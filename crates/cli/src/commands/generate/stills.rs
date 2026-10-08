//! The stills half of `scorsese generate`: Gemini, and what it costs.
//!
//! Shaped like [`lines`](super::lines), for its reason: a picture comes back on
//! the call that asked for it — unless it was ordered in a half-price batch
//! (#894), which a later run or the sweep collects.

use std::path::Path;

use anyhow::Result;
use scorsese_core::{AssetId, Project};
use scorsese_providers::credentials::{Budget, Provider, resolve};
use scorsese_providers::image::{GeminiProvider, Outcome, batch, collect, generate};

/// Draws every still that needs it. The key — the same Gemini key Veo takes —
/// is resolved only when this pass has something to do.
pub(super) fn pass(
    project: &mut Project,
    project_dir: &Path,
    budget: Budget,
) -> Result<Vec<(AssetId, Outcome)>> {
    let key = resolve(Provider::Gemini)?;
    let provider = GeminiProvider::new(&key.secret);
    Ok(generate(project, project_dir, &provider, budget)?)
}

/// Orders every still that needs it as a half-price batch, collecting any
/// earlier batch first.
pub(super) fn order(
    project: &mut Project,
    project_dir: &Path,
    budget: Budget,
) -> Result<Vec<(AssetId, Outcome)>> {
    let key = resolve(Provider::Gemini)?;
    let provider = GeminiProvider::new(&key.secret);
    Ok(batch(project, project_dir, &provider, budget)?)
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
