//! The shots half of the `generate` tool: Veo, and what it costs.
//!
//! Split from [`super`] because there are two providers now, and one file
//! holding both would be a file nobody reads to answer a question about
//! either.

use std::time::Duration;

use scorsese_core::Project;
use scorsese_providers::credentials::{Budget, Provider, resolve};
use scorsese_providers::video::{
    Outcome, RETENTION_DAYS, Run, VeoProvider, collect, generate_waiting,
};

/// Realises every sketched shot, waiting up to `patience`.
///
/// The key is resolved **here**, and only when this pass has something to do —
/// a project whose only generated assets are narration must never be asked for
/// a Veo key.
pub(super) fn pass(
    project: &mut Project,
    dir: &std::path::Path,
    budget: Budget,
    patience: Duration,
    collecting: bool,
) -> Result<Run, String> {
    let key = resolve(Provider::Gemini).map_err(|error| format!("{error}"))?;
    let provider = VeoProvider::new(&key.secret);
    let outcome = if collecting {
        // A sweep spends nothing by construction, so its total is zero.
        collect(project, dir, &provider).map(|outcomes| Run {
            outcomes,
            spent_cents: 0,
        })
    } else {
        generate_waiting(
            project,
            dir,
            &provider,
            budget,
            patience,
            std::thread::sleep,
        )
    };
    outcome.map_err(|error| format!("{error}"))
}

/// What the shots in a run read as.
pub(super) fn said(run: &Run, lines: &mut Vec<String>) {
    for (id, outcome) in &run.outcomes {
        lines.push(format!("{id}: {}", one(outcome)));
    }
    let in_flight = run.in_flight();
    if in_flight > 0 {
        lines.push(format!(
            "{in_flight} still generating. The tickets are in project.json, so nothing is \
             lost — call again with collect: true to pick them up."
        ));
    }
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
