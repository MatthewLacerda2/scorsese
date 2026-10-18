//! The tool that spends money.
//!
//! Every other tool here costs a subprocess at most. This one hands briefs to a
//! provider and is billed for them, which is why its description says so in the
//! first sentence, why a call without `confirm` only quotes — see
//! [`confirm`](super::confirm) — and why the reply always names what the run is
//! estimated to have cost.
//!
//! **A brief already generated is never sent again.** The cache is keyed on a
//! hash of everything the brief asks for, so calling this twice by mistake — or
//! after a dropped connection, which is the likelier case — costs nothing the
//! second time.
//!
//! **Three passes, one ceiling, one total.** Shots, stills and narration are
//! separate passes because they are separate models and vendors, but the budget
//! is threaded from each into the next and the totals are added — a ceiling each pass
//! checked on its own would be worth twice what somebody set.

mod said;

use std::time::Duration;

use schemars::JsonSchema;
use scorsese_core::Project;
use scorsese_core::placing::Shortened;
use scorsese_providers::credentials::Budget;
use scorsese_providers::image::Order;
use scorsese_providers::passes::{self, Done, Halt, Keyed, Step};
use scorsese_providers::quote::{self, generation};
use scorsese_providers::video::WAIT_FOR;
use scorsese_render::Ffprobe;
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, ProjectDir};
use crate::tools::confirm::{self, Token};
use crate::tools::inspect::{load, save};
use crate::tools::{Costs, Reply, Tool};

/// Realising generated video, stills and narration.
pub(crate) struct Generate;

/// What `generate` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    confirm: Option<Token>,
    /// Collect whatever has finished and submit nothing at all. What to call on
    /// returning to a project with shots in flight or stills waiting in a
    /// batch — it cannot spend anything. Narration is never in flight.
    #[serde(default)]
    collect: bool,
    /// Order the stills as a half-price batch, ready within 24 hours, instead
    /// of drawing them now. Only when whoever is paying chose to wait: it
    /// trades their time for half the price. Stills only — refused when a
    /// shot or a line would be sent too. A later call collects them.
    #[serde(default)]
    batch: bool,
    /// How long to wait for video before detaching and leaving the rest to be
    /// collected later. Default 300. Nothing is lost by detaching; the tickets
    /// are in the document.
    wait_seconds: Option<u64>,
}

impl args::Arguments for Arguments {}

impl Tool for Generate {
    fn name(&self) -> &'static str {
        "generate"
    }

    fn description(&self) -> &'static str {
        "Realise the sketched briefs — this costs money (voice_design is the only other \
         tool that does), and it quotes before it spends. A shot or still a stock library \
         could have is searched with stock_search first (guide stock): generate one only \
         when it must be custom-made or the person asked for a generation. Narration is \
         cheap; generate a line whenever it is needed. Called without confirm it sends nothing and needs no \
         key: it answers with what each generated_video (Veo), generated_image \
         (Gemini) and generated_audio (ElevenLabs) brief would cost, and a token. Before the \
         first quote, every brief it covers is storyboarded in words in the script \
         (guide prompts), and agreed with the person whenever they are there. Show that quote to whoever is \
         paying; only a second call with confirm set to the token spends, and only on \
         exactly the briefs quoted — edit one in between and the call is refused and \
         must be quoted again. A run with nothing to pay for (everything already \
         generated, or shots only waiting to be collected) needs no token. A brief \
         already generated is never sent again. When stills are worth a dollar or more \
         the quote also prices them in a batch: half price, ready within 24 hours. Put \
         that choice to whoever is paying and never pick the wait for them; if they \
         take it, quote and confirm again with batch set — stills only, so shots and \
         lines go in a call of their own. Video takes minutes, so a confirmed \
         run waits a while and then detaches: whatever is still going has its ticket \
         written into project.json, and calling with collect picks it up — collect \
         never spends and never needs a token, and it picks up batched stills too. \
         Stills drawn now and narration come back on the same call; a still whose reference is a generated_image not yet \
         generated is reported and drawn on the next call, even when that reference is drawn earlier in \
         the same run, because the quote did not price it. A shot whose image is a still not drawn yet is \
         likewise reported and skipped. A line with no voice chosen yet is reported and skipped rather than \
         failing the run. A run that stops part-way (Veo at its limit of shots in flight, the network gone, \
         the ceiling reached) still lists what it sent and names what it did not; nothing after the stop is \
         attempted, so call again once the shots in flight finish. A shot or line that comes out shorter than a clip laid out \
         over its sketch shortens that clip to it, and the reply names each one — the \
         gap left after it is yours to close. Every figure is our own arithmetic over \
         published rates, never a bill."
    }

    fn costs(&self) -> Costs {
        Costs::Money
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let mut project = load(dir)?;
        let collecting = arguments.collect;
        let order = if arguments.batch {
            Order::Batch
        } else {
            Order::Now
        };
        // Collecting submits nothing by construction, so there is nothing to
        // agree to. Everything else is quoted, and goes ahead only on a token
        // bound to that quote — or on a quote with nothing in it to pay for.
        if !collecting {
            let quote = match order {
                Order::Now => generation(&project, dir),
                Order::Batch => quote::batch(&project, dir),
            }
            .map_err(|error| format!("{error}"))?;
            let offer = match order {
                Order::Now => quote::offer(&project, dir).map_err(|error| format!("{error}"))?,
                Order::Batch => None,
            };
            let notes: Vec<String> = offer.iter().map(quote::Offer::says).collect();
            if let Some(quoted) =
                confirm::gate(dir, arguments.confirm.as_ref(), &quote, self.name(), &notes)?
            {
                return Ok(quoted);
            }
        }

        let budget = Budget::for_project(&project, dir)
            .map_err(|error| format!("nothing was sent: {error}"))?;
        let patience = arguments.wait_seconds.map_or(WAIT_FOR, Duration::from_secs);

        let (done, outcome) = if collecting {
            passes::collect(&mut project, dir, &Keyed)
        } else {
            let asked = passes::Asked {
                budget,
                patience,
                order,
            };
            passes::run(&mut project, dir, &asked, &Keyed)
        };

        // Saved whatever happened, and before the error is reported: a ticket
        // written just before a failure is the only record that money was
        // spent, and dropping it means paying for that work twice.
        save(&project, dir)?;
        // A run that stopped after doing something answers with what it did
        // and then why it stopped: an error alone, after spending, reads as
        // nothing having happened, and is the reply most likely to be retried
        // blindly (#1057). Only a run that did nothing at all is an error.
        let stopped = match outcome {
            Err(halt) if done.is_empty() => return Err(says(halt)),
            Err(halt) => Some(says(halt)),
            Ok(()) => None,
        };

        let shortened = measure(&mut project, dir, &done)?;
        let mut reply = said::run(&done, &project, dir);
        for one in shortened.iter().map(Shortened::says).chain(stopped) {
            reply.push('\n');
            reply.push_str(&one);
        }
        Ok(reply.into())
    }
}

/// Why a run ended early, in words. A pass after the shots that could not
/// start — no key — stopped the run, and says so; one that started and
/// stopped part-way says so itself.
fn says(halt: Halt) -> String {
    match halt {
        Halt::Key {
            step: Step::Stills | Step::Lines | Step::Tracks,
            error,
        } => format!("The run stopped: {error} — nothing after it was attempted."),
        Halt::Key { error, .. } => error.to_string(),
        Halt::Stopped(says) => says,
        Halt::Shots(error) => error.to_string(),
        Halt::Stills(error) => error.to_string(),
    }
}

/// Measures what has just been generated.
///
/// **A generation has no measured shape until something looks**, and neither
/// kind of brief can supply one. How long a line takes depends on the words;
/// whether Veo put an engine note under a shot is not in the prompt either, and
/// it routinely does. `scorsese-providers` can answer neither — it has no
/// decoder and must not grow one. The mix reads `duration_seconds` and
/// `audio_channels`, so a generation nobody probed is a line the mix skips and
/// a shot that plays silent.
///
/// A failure to measure is **not** a failure of the run: the media exists and
/// has been paid for, and probing it again later is free. Saying so and
/// carrying on beats reporting a spend as an error.
///
/// **Measuring is what can make the document refuse to load**: a clip laid
/// out over a sketch may outlast what came back, and the length just written
/// is what says so. So the clips of what landed are shortened to it before the
/// save ([`Done::measure`], #825), and the ones that were are handed
/// back for the reply.
fn measure(
    project: &mut Project,
    dir: &std::path::Path,
    done: &Done,
) -> Result<Vec<Shortened>, String> {
    if !done.unmeasured() {
        return Ok(Vec::new());
    }
    let Ok(probe) = Ffprobe::discover() else {
        return Ok(Vec::new());
    };
    let shortened = done.measure(project, dir, &probe);
    save(project, dir)?;
    Ok(shortened)
}
