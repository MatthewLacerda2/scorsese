//! The stills half of the `generate` tool: Gemini, and what it costs.
//!
//! Shaped like [`lines`](super::lines): a picture comes back on the call —
//! unless it was ordered in a half-price batch (#894), which a later call or a
//! collect picks up.

use scorsese_core::{AssetId, Project};
use scorsese_providers::credentials::{Budget, Provider, resolve};
use scorsese_providers::image::{GeminiProvider, Outcome, batch, collect, generate};

/// What one stills run produced.
pub(super) type Drawn = Vec<(AssetId, Outcome)>;

/// Draws every still that needs it. The key — the Gemini key Veo also takes —
/// is resolved only when this pass has something to do.
pub(super) fn pass(
    project: &mut Project,
    dir: &std::path::Path,
    budget: Budget,
) -> Result<Drawn, String> {
    let key = resolve(Provider::Gemini).map_err(|error| format!("{error}"))?;
    let provider = GeminiProvider::new(&key.secret);
    generate(project, dir, &provider, budget).map_err(|error| format!("{error}"))
}

/// Orders every still that needs it as a half-price batch, collecting any
/// earlier batch first.
pub(super) fn order(
    project: &mut Project,
    dir: &std::path::Path,
    budget: Budget,
) -> Result<Drawn, String> {
    let key = resolve(Provider::Gemini).map_err(|error| format!("{error}"))?;
    let provider = GeminiProvider::new(&key.secret);
    batch(project, dir, &provider, budget).map_err(|error| format!("{error}"))
}

/// Collects every still waiting in a batch, and orders nothing. Asks for a
/// key only when one is waiting.
pub(super) fn sweep(project: &mut Project, dir: &std::path::Path) -> Result<Drawn, String> {
    let waiting = project.assets.iter().any(|asset| {
        asset.kind == scorsese_core::AssetKind::GeneratedImage && asset.operation.is_some()
    });
    if !waiting {
        return Ok(Vec::new());
    }
    let key = resolve(Provider::Gemini).map_err(|error| format!("{error}"))?;
    let provider = GeminiProvider::new(&key.secret);
    collect(project, dir, &provider).map_err(|error| format!("{error}"))
}

/// What the stills in a run read as.
pub(super) fn said(drawn: &Drawn, lines: &mut Vec<String>) {
    for (id, outcome) in drawn {
        lines.push(format!("{id}: {}", outcome.says()));
    }
}

/// What the stills spent.
pub(super) fn spent(drawn: &Drawn) -> u64 {
    drawn.iter().map(|(_, outcome)| outcome.spent_cents()).sum()
}

/// Whether a still landed that nothing has measured yet.
pub(super) fn landed(drawn: &Drawn) -> bool {
    drawn.iter().any(|(_, outcome)| outcome.landed())
}
