//! The stills half of `scorsese generate`: Gemini, and what it costs.
//!
//! Shaped like [`lines`](super::lines), for its reason: a picture comes back on
//! the call that asked for it, so nothing is in flight and there is no sweep.

use std::path::Path;

use anyhow::Result;
use scorsese_core::{AssetId, Project};
use scorsese_providers::credentials::{Budget, Provider, resolve};
use scorsese_providers::image::{GeminiProvider, Outcome, generate};

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

/// One line per still.
pub(super) fn report(outcomes: &[(AssetId, Outcome)]) {
    for (id, outcome) in outcomes {
        println!("{id:<24} {}", outcome.says());
    }
}
