//! The stills half of `scorsese generate`: Gemini, and what it costs.
//!
//! Shaped like [`lines`](super::lines), for its reason: a picture comes back on
//! the call that asked for it — unless it was ordered in a half-price batch
//! (#894), which a later run or the sweep collects.

use std::path::Path;

use anyhow::Result;
use scorsese_core::{AssetId, Project};
use scorsese_providers::credentials::{Budget, Provider, resolve};
use scorsese_providers::image::{GeminiProvider, Outcome, Stopped, batch, collect, generate};

/// Draws every still that needs it, into `drawn`. The key — the same Gemini
/// key Veo takes — is resolved only when this pass has something to do.
///
/// **A run that stops part-way still fills `drawn`** with the stills it drew
/// before stopping, and the error says in words how many went (#1153).
pub(super) fn pass(
    project: &mut Project,
    project_dir: &Path,
    budget: Budget,
    drawn: &mut Vec<(AssetId, Outcome)>,
) -> Result<()> {
    let key = resolve(Provider::Gemini)?;
    let provider = GeminiProvider::new(&key.secret);
    kept(generate(project, project_dir, &provider, budget), drawn)
}

/// Orders every still that needs it as a half-price batch, collecting any
/// earlier batch first — into `ordered`, as [`pass`] does.
pub(super) fn order(
    project: &mut Project,
    project_dir: &Path,
    budget: Budget,
    ordered: &mut Vec<(AssetId, Outcome)>,
) -> Result<()> {
    let key = resolve(Provider::Gemini)?;
    let provider = GeminiProvider::new(&key.secret);
    kept(batch(project, project_dir, &provider, budget), ordered)
}

/// What a run did, into `drawn` — a stopped one's included — and why it
/// stopped, in words.
fn kept(
    run: Result<Vec<(AssetId, Outcome)>, Stopped>,
    drawn: &mut Vec<(AssetId, Outcome)>,
) -> Result<()> {
    match run {
        Ok(run) => *drawn = run,
        Err(stopped) => {
            let says = stopped.says();
            *drawn = stopped.run;
            return Err(anyhow::anyhow!(says));
        }
    }
    Ok(())
}

/// Collects every still waiting in a batch, and orders nothing. Asks for a
/// key only when one is waiting.
pub(super) fn sweep(project: &mut Project, project_dir: &Path) -> Result<Vec<(AssetId, Outcome)>> {
    let waiting = project.assets.iter().any(|asset| {
        asset.kind == scorsese_core::AssetKind::GeneratedImage && asset.operation.is_some()
    });
    if !waiting {
        return Ok(Vec::new());
    }
    let key = resolve(Provider::Gemini)?;
    let provider = GeminiProvider::new(&key.secret);
    Ok(collect(project, project_dir, &provider)?)
}

/// One line per still.
pub(super) fn report(outcomes: &[(AssetId, Outcome)]) {
    for (id, outcome) in outcomes {
        println!("{id:<24} {}", outcome.says());
    }
}
