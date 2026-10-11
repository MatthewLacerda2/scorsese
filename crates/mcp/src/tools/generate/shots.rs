//! The shots half of the `generate` tool: Veo, and what it costs.
//!
//! Split from [`super`] because there are two providers now, and one file
//! holding both would be a file nobody reads to answer a question about
//! either.

use scorsese_core::Project;
use scorsese_providers::credentials::{Provider, resolve};
use scorsese_providers::video::{Run, VeoProvider, collect, generate_waiting};

/// Realises every sketched shot, waiting up to `patience`, into `shots`.
///
/// The key is resolved **here**, and only when this pass has something to do —
/// a project whose only generated assets are narration must never be asked for
/// a Veo key.
///
/// **A run that stops part-way still fills `shots`** with what it sent before
/// stopping, and the error is the whole story in words — so many sent, so many
/// not — rather than the vendor's body (#1057).
pub(super) fn pass(
    project: &mut Project,
    dir: &std::path::Path,
    asked: &super::Passes,
    shots: &mut Run,
) -> Result<(), String> {
    let key = resolve(Provider::Gemini).map_err(|error| format!("{error}"))?;
    let provider = VeoProvider::new(&key.secret);
    if asked.collecting {
        // A sweep spends nothing by construction, so its total is zero.
        shots.outcomes = collect(project, dir, &provider).map_err(|error| format!("{error}"))?;
        return Ok(());
    }
    match generate_waiting(
        project,
        dir,
        &provider,
        asked.budget,
        asked.patience,
        std::thread::sleep,
    ) {
        Ok(run) => *shots = run,
        Err(stopped) => {
            let says = stopped.says();
            *shots = stopped.run;
            return Err(says);
        }
    }
    Ok(())
}

/// What the shots in a run read as.
pub(super) fn said(run: &Run, lines: &mut Vec<String>) {
    for (id, outcome) in &run.outcomes {
        lines.push(format!("{id}: {}", outcome.says()));
    }
    let in_flight = run.in_flight();
    if in_flight > 0 {
        lines.push(format!(
            "{in_flight} still generating. The tickets are in project.json, so nothing is \
             lost — call again with collect: true to pick them up."
        ));
    }
}
