//! The stills half of the `generate` tool: Gemini, and what it costs.
//!
//! Shaped like [`lines`](super::lines): a picture comes back on the call, so
//! nothing is in flight and there is nothing to collect.

use scorsese_core::{AssetId, Project};
use scorsese_providers::credentials::{Budget, Provider, resolve};
use scorsese_providers::image::{GeminiProvider, Outcome, generate};

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
    drawn
        .iter()
        .any(|(_, outcome)| matches!(outcome, Outcome::Generated { .. }))
}
