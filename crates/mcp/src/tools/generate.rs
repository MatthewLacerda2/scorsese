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

use std::time::Duration;

use scorsese_core::Project;
use scorsese_providers::credentials::{Budget, Provider, Settings, resolve};
use scorsese_providers::prices::{dollars, estimate};
use scorsese_providers::video::{
    Outcome, RETENTION_DAYS, Run, VeoProvider, WAIT_FOR, collect, generate_waiting,
};
use serde_json::Value;

use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir, project_property};

/// Realising generated video.
pub(crate) struct Generate;

impl Tool for Generate {
    fn name(&self) -> &'static str {
        "generate"
    }

    fn description(&self) -> &'static str {
        "Realise the sketched shots — the one tool here that costs money. Each \
         generated_video asset whose brief has not been paid for is handed to \
         Veo; the reply says what each shot did and what the run is estimated \
         to have cost. A brief already generated is never sent again, so \
         calling this twice costs nothing the second time. Pass dry_run to be \
         quoted a price and send nothing. Generation takes minutes, so this \
         waits a while and then detaches: whatever is still going has its \
         ticket written into project.json, and calling again — or with \
         collect — picks it up, even from another machine. Every figure is our \
         own arithmetic over published rates, never a bill."
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
                                    in flight — it cannot spend anything."
                },
                "wait_seconds": {
                    "type": "integer",
                    "description": "How long to wait before detaching and leaving the rest \
                                    to be collected later. Default 300. Nothing is lost by \
                                    detaching; the tickets are in the document."
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

        let key = resolve(Provider::Gemini).map_err(|error| format!("{error}"))?;
        let provider = VeoProvider::new(&key.secret);
        let settings = Settings::load().unwrap_or_default();
        let budget = Budget::from_settings(&settings, spent_so_far(&project));

        let outcome = if flag(arguments, "collect") {
            // A sweep spends nothing by construction, so its total is zero.
            collect(&mut project, &dir, &provider).map(|outcomes| Run {
                outcomes,
                spent_cents: 0,
            })
        } else {
            generate_waiting(
                &mut project,
                &dir,
                &provider,
                budget,
                patience(arguments)?,
                std::thread::sleep,
            )
        };
        // Saved whatever happened, and before the error is reported: a ticket
        // written just before a failure is the only record that money was
        // spent, and dropping it means paying for that work twice.
        project
            .save(&dir)
            .map_err(|error| format!("saving the project: {error}"))?;
        Ok(said(&outcome.map_err(|error| format!("{error}"))?).into())
    }
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

/// What the assets already say has been spent on them.
fn spent_so_far(project: &Project) -> u64 {
    project
        .assets
        .iter()
        .filter_map(|asset| asset.estimated_cost_cents)
        .sum()
}

/// What a run would cost, without a key and without sending anything.
fn quote(project: &Project) -> Result<Reply, String> {
    let mut lines = Vec::new();
    let mut total = 0;
    for asset in project
        .assets
        .iter()
        .filter(|asset| asset.kind.is_prompted())
    {
        let request = asset.video_request();
        let priced = estimate(&request).map_err(|error| format!("{error}"))?;
        lines.push(format!(
            "{}: {} — {}s of {} at {}",
            asset.id,
            dollars(priced.cents),
            priced.seconds,
            request.model.as_str(),
            request.resolution.as_str()
        ));
        total += priced.cents;
    }
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
fn said(run: &Run) -> String {
    if run.outcomes.is_empty() {
        return String::from("Nothing to generate: no generated_video assets in this project.");
    }
    let mut lines: Vec<String> = run
        .outcomes
        .iter()
        .map(|(id, outcome)| format!("{id}: {}", one(outcome)))
        .collect();

    lines.push(format!(
        "About {} spent on this run — our calculation, never a bill.",
        dollars(run.spent_cents)
    ));
    let in_flight = run.in_flight();
    if in_flight > 0 {
        lines.push(format!(
            "{in_flight} still generating. The tickets are in project.json, so nothing is \
             lost — call again with collect: true to pick them up."
        ));
    }
    lines.join("\n")
}

/// What one shot's outcome reads as.
fn one(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Cached { path } => format!("already generated — {path}"),
        Outcome::Queued { operation, .. } => format!("queued — {operation}"),
        Outcome::Waiting { operation, .. } => format!("still generating — {operation}"),
        Outcome::Expired { queued_at, .. } => format!(
            "queued {} and past the {RETENTION_DAYS}-day window — the video is gone and the \
             shot has to be asked for again",
            queued_at
                .as_ref()
                .map_or_else(|| String::from("at some point"), ToString::to_string)
        ),
        Outcome::Generated { path, bytes, .. } => format!("generated — {path} ({bytes} bytes)"),
        Outcome::Failed { message } => format!("refused — {message}"),
    }
}
