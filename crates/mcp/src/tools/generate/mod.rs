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

mod lines;
mod shots;
mod stills;

use std::path::Path;
use std::time::Duration;

use schemars::JsonSchema;
use scorsese_core::placing::{self, Shortened};
use scorsese_core::{AssetId, Project, Reprobe, probe_assets};
use scorsese_providers::credentials::{Budget, Settings};
use scorsese_providers::image::Order;
use scorsese_providers::prices::dollars;
use scorsese_providers::quote::{self, generation};
use scorsese_providers::video::{Run, WAIT_FOR};
use scorsese_providers::{image, speech, spending, video};
use scorsese_render::Ffprobe;
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, ProjectDir};
use crate::tools::confirm::{self, Token};
use crate::tools::inspect::load;
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
        "Realise the sketched briefs — the one tool here that costs money, and it \
         quotes before it spends. Called without confirm it sends nothing and needs no \
         key: it answers with what each generated_video (Veo), generated_image \
         (Gemini) and generated_audio (ElevenLabs) brief would cost, and a token. Show that quote to whoever is \
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
         generated is reported and drawn on the next call. A line with no voice chosen yet is reported and skipped rather than \
         failing the run. A shot or line that comes out shorter than a clip laid out \
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

        let settings = Settings::load().unwrap_or_default();
        let budget = Budget::from_settings(&settings, spent_so_far(&project, dir));
        let patience = arguments.wait_seconds.map_or(WAIT_FOR, Duration::from_secs);

        let mut shots = Run {
            outcomes: Vec::new(),
            spent_cents: 0,
        };
        let mut drawn = stills::Drawn::new();
        let mut spoken = lines::Spoken::new();
        let outcome = run(
            &mut project,
            dir,
            Passes {
                budget,
                patience,
                collecting,
                order,
            },
            &mut shots,
            &mut drawn,
            &mut spoken,
        );

        // Saved whatever happened, and before the error is reported: a ticket
        // written just before a failure is the only record that money was
        // spent, and dropping it means paying for that work twice.
        project
            .save(dir)
            .map_err(|error| format!("saving the project: {error}"))?;
        outcome?;

        let landed = landed(&shots, &spoken);
        let shortened = measure(&mut project, dir, &landed, stills::landed(&drawn))?;
        let mut reply = said(&shots, &drawn, &spoken, dir);
        for one in &shortened {
            reply.push('\n');
            reply.push_str(&one.says());
        }
        Ok(reply.into())
    }
}

/// What one call asked for, beyond the project.
struct Passes {
    budget: Budget,
    patience: Duration,
    collecting: bool,
    order: Order,
}

/// Both passes, each run only if it has something to do.
fn run(
    project: &mut Project,
    dir: &std::path::Path,
    asked: Passes,
    shots: &mut Run,
    drawn: &mut stills::Drawn,
    spoken: &mut lines::Spoken,
) -> Result<(), String> {
    // Whether a pass runs at all is each provider's `pending`, which consults
    // the **current brief's** output file — never the asset's recorded `path`.
    // The recorded path is the previous generation's file: after an edit it
    // still exists and still resolves, and consulting it is how a stale shot
    // used to be skipped as *nothing to do*.
    // A batch is stills alone: its quote was refused had anything else been
    // waiting to be sent.
    if asked.order == Order::Batch {
        if image::pending(project, dir) {
            *drawn = stills::order(project, dir, asked.budget)?;
        }
        return Ok(());
    }
    if video::pending(project, dir) {
        *shots = shots::pass(project, dir, asked.budget, asked.patience, asked.collecting)?;
    }
    // Collecting submits nothing by definition. Stills waiting in a batch are
    // collected; narration is never in flight, so there is nothing for its
    // pass to collect and asking for a key would be asking for one to do
    // nothing with.
    if asked.collecting {
        *drawn = stills::sweep(project, dir)?;
        return Ok(());
    }
    if image::pending(project, dir) {
        *drawn = stills::pass(project, dir, asked.budget.spend(shots.spent_cents))?;
    }
    if speech::pending(project, dir) {
        let committed = shots.spent_cents + stills::spent(drawn);
        *spoken = lines::pass(project, dir, asked.budget.spend(committed))?;
    }
    Ok(())
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
/// save ([`placing::fit_to_sources`], #825), and the ones that were are handed
/// back for the reply.
fn measure(
    project: &mut Project,
    dir: &std::path::Path,
    landed: &[AssetId],
    drawn: bool,
) -> Result<Vec<Shortened>, String> {
    if landed.is_empty() && !drawn {
        return Ok(Vec::new());
    }
    let Ok(probe) = Ffprobe::discover() else {
        return Ok(Vec::new());
    };
    probe_assets(project, dir, &probe, Reprobe::Skip);
    let shortened = placing::fit_to_sources(project, landed);
    project
        .save(dir)
        .map_err(|error| format!("saving the project: {error}"))?;
    Ok(shortened)
}

/// What arrived on disk with a length nothing has measured yet.
///
/// A cache hit is not one of them: its file was already there, and whatever ran
/// when it first landed has had every chance to look at it.
fn landed(shots: &Run, spoken: &lines::Spoken) -> Vec<AssetId> {
    let shot = shots.outcomes.iter().filter(|(_, outcome)| {
        matches!(
            outcome,
            scorsese_providers::video::Outcome::Generated { .. }
        )
    });
    let line = spoken.iter().filter(|(_, outcome)| {
        matches!(
            outcome,
            scorsese_providers::speech::Outcome::Generated { .. }
        )
    });
    shot.map(|(id, _)| id.clone())
        .chain(line.map(|(id, _)| id.clone()))
        .collect()
}

/// What this project has already spent, against the ceiling.
///
/// The assets **and** the designed-voice ledger, because a ceiling that counted
/// only one of them would be wrong by however much the other holds — designing
/// a voice is real money at the same vendor. They stay separate figures
/// wherever they are *reported*; see [`spending`].
fn spent_so_far(project: &Project, root: &Path) -> u64 {
    spending::so_far(project, root).total()
}

/// What the run reads as.
fn said(shots: &Run, drawn: &stills::Drawn, spoken: &lines::Spoken, dir: &Path) -> String {
    if shots.outcomes.is_empty() && drawn.is_empty() && spoken.is_empty() {
        return String::from("Nothing to generate: no prompted assets in this project.");
    }
    let mut lines = Vec::new();
    shots::said(shots, &mut lines);
    stills::said(drawn, &mut lines);
    lines::said(spoken, dir, &mut lines);

    let spent = shots.spent_cents
        + stills::spent(drawn)
        + spoken.iter().map(|(_, o)| o.spent_cents()).sum::<u64>();
    lines.push(format!(
        "About {} spent on this run — our calculation, never a bill.",
        dollars(spent)
    ));
    lines.join("\n")
}
