//! The tool that spends money.
//!
//! Every other tool here costs a subprocess at most. This one hands briefs to a
//! provider and is billed for them, which is why its description says so in the
//! first sentence, why `dry_run` exists, and why the reply always names what
//! the run is estimated to have cost.
//!
//! **A brief already generated is never sent again.** The cache is keyed on a
//! hash of everything the brief asks for, so calling this twice by mistake — or
//! after a dropped connection, which is the likelier case — costs nothing the
//! second time.
//!
//! **Two providers, one ceiling, one total.** Shots and narration are separate
//! passes because they are separate vendors, but the budget is threaded from
//! the first into the second and the totals are added — a ceiling each pass
//! checked on its own would be worth twice what somebody set.

mod lines;
mod shots;

use std::path::Path;
use std::time::Duration;

use scorsese_core::{Asset, AssetKind, Project, Reprobe, probe_assets};
use scorsese_providers::credentials::{Budget, Settings};
use scorsese_providers::prices::dollars;
use scorsese_providers::spending;
use scorsese_providers::video::{Run, WAIT_FOR};
use scorsese_render::Ffprobe;
use serde_json::Value;

use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir, project_property};

/// Realising generated video and narration.
pub(crate) struct Generate;

impl Tool for Generate {
    fn name(&self) -> &'static str {
        "generate"
    }

    fn description(&self) -> &'static str {
        "Realise the sketched briefs — the one tool here that costs money. Each \
         generated_video asset whose brief has not been paid for is handed to \
         Veo, and each generated_audio asset to ElevenLabs; the reply says what \
         each did and what the run is estimated to have cost. A brief already \
         generated is never sent again, so calling this twice costs nothing the \
         second time. Pass dry_run to be quoted a price and send nothing. Video \
         takes minutes, so this waits a while and then detaches: whatever is \
         still going has its ticket written into project.json, and calling \
         again — or with collect — picks it up, even from another machine. \
         Narration comes back on the same call and is never left in flight. A \
         line with no voice chosen yet is reported and skipped rather than \
         failing the run. Every figure is our own arithmetic over published \
         rates, never a bill."
    }

    fn costs(&self) -> Costs {
        Costs::Money
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "dry_run": {
                    "type": "boolean",
                    "description": "Say what a run would cost and send nothing. Needs no \
                                    key. Worth doing before any run somebody has not \
                                    agreed to the price of."
                },
                "collect": {
                    "type": "boolean",
                    "description": "Collect whatever has finished and submit nothing at \
                                    all. What to call on returning to a project with shots \
                                    in flight — it cannot spend anything. Narration is \
                                    never in flight, so this concerns video only."
                },
                "wait_seconds": {
                    "type": "integer",
                    "description": "How long to wait for video before detaching and leaving \
                                    the rest to be collected later. Default 300. Nothing is \
                                    lost by detaching; the tickets are in the document."
                }
            },
            "required": ["project"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        if flag(arguments, "dry_run") {
            return quote(&project);
        }

        let settings = Settings::load().unwrap_or_default();
        let budget = Budget::from_settings(&settings, spent_so_far(&project, &dir));
        let collecting = flag(arguments, "collect");
        let patience = patience(arguments)?;

        let mut shots = Run {
            outcomes: Vec::new(),
            spent_cents: 0,
        };
        let mut spoken = lines::Spoken::new();
        let outcome = run(
            &mut project,
            &dir,
            Passes {
                budget,
                patience,
                collecting,
            },
            &mut shots,
            &mut spoken,
        );

        // Saved whatever happened, and before the error is reported: a ticket
        // written just before a failure is the only record that money was
        // spent, and dropping it means paying for that work twice.
        project
            .save(&dir)
            .map_err(|error| format!("saving the project: {error}"))?;
        outcome?;

        measure(&mut project, &dir, &spoken)?;
        Ok(said(&shots, &spoken).into())
    }
}

/// What one call asked for, beyond the project.
struct Passes {
    budget: Budget,
    patience: Duration,
    collecting: bool,
}

/// Both passes, each run only if it has something to do.
fn run(
    project: &mut Project,
    dir: &std::path::Path,
    asked: Passes,
    shots: &mut Run,
    spoken: &mut lines::Spoken,
) -> Result<(), String> {
    if wants(project, dir, AssetKind::GeneratedVideo) {
        *shots = shots::pass(project, dir, asked.budget, asked.patience, asked.collecting)?;
    }
    // Collecting submits nothing by definition, and narration is never in
    // flight — so there is nothing for this pass to collect and asking for a
    // key would be asking for one to do nothing with.
    if !asked.collecting && wants(project, dir, AssetKind::GeneratedAudio) {
        *spoken = lines::pass(project, dir, asked.budget.spend(shots.spent_cents))?;
    }
    Ok(())
}

/// Whether this kind has anything left to do, and so whether its key is needed.
///
/// True when a brief of that kind has no file behind it yet, or has work in
/// flight. Asked of the **disk** and not only the document: an asset can say
/// `generated` and have lost its file, and that one does need generating again
/// whatever the state field claims.
fn wants(project: &Project, root: &std::path::Path, kind: AssetKind) -> bool {
    project
        .assets
        .iter()
        .filter(|asset| asset.kind == kind)
        .any(|asset| asset.operation.is_some() || !is_present(asset, root))
}

/// Whether this asset's media is actually on disk.
fn is_present(asset: &Asset, root: &std::path::Path) -> bool {
    asset
        .path
        .as_ref()
        .is_some_and(|path| path.resolve(root).is_file())
}

/// Measures what was just spoken.
///
/// **Narration has no length until something looks.** A shot comes back exactly
/// as long as its request asked, so the provider fills that in from the brief.
/// How long a line takes depends on the words, and `scorsese-providers` has no
/// decoder to measure an MP3 with — nor should it grow one. The mix reads
/// `duration_seconds`, so an unprobed line is a line the mix skips.
///
/// A failure to measure is **not** a failure of the run: the audio exists and
/// has been paid for, and probing it again later is free. Saying so and
/// carrying on beats reporting a spend as an error.
fn measure(
    project: &mut Project,
    dir: &std::path::Path,
    spoken: &lines::Spoken,
) -> Result<(), String> {
    let spoke = spoken.iter().any(|(_, outcome)| {
        matches!(
            outcome,
            scorsese_providers::speech::Outcome::Generated { .. }
        )
    });
    if !spoke {
        return Ok(());
    }
    let Ok(probe) = Ffprobe::discover() else {
        return Ok(());
    };
    probe_assets(project, dir, &probe, Reprobe::Skip);
    project
        .save(dir)
        .map_err(|error| format!("saving the project: {error}"))
}

/// A boolean argument, absent meaning false.
fn flag(arguments: &Value, name: &str) -> bool {
    arguments
        .get(name)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// How long to wait before detaching.
fn patience(arguments: &Value) -> Result<Duration, String> {
    match arguments.get("wait_seconds") {
        None | Some(Value::Null) => Ok(WAIT_FOR),
        Some(value) => value
            .as_u64()
            .map(Duration::from_secs)
            .ok_or_else(|| format!("wait_seconds: {value} is not a number of seconds")),
    }
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

/// What a run would cost, without a key and without sending anything.
fn quote(project: &Project) -> Result<Reply, String> {
    let mut lines = Vec::new();
    let total = shots::quote(project, &mut lines)? + lines::quote(project, &mut lines)?;
    if lines.is_empty() {
        return Ok("Nothing to generate: this project has no prompted assets.".into());
    }
    lines.push(format!(
        "About {} for the whole run — our arithmetic over published rates, never a bill.",
        dollars(total)
    ));
    Ok(lines.join("\n").into())
}

/// What the run reads as.
fn said(shots: &Run, spoken: &lines::Spoken) -> String {
    if shots.outcomes.is_empty() && spoken.is_empty() {
        return String::from("Nothing to generate: no prompted assets in this project.");
    }
    let mut lines = Vec::new();
    shots::said(shots, &mut lines);
    lines::said(spoken, &mut lines);

    let spent = shots.spent_cents + spoken.iter().map(|(_, o)| o.spent_cents()).sum::<u64>();
    lines.push(format!(
        "About {} spent on this run — our calculation, never a bill.",
        dollars(spent)
    ));
    lines.join("\n")
}
