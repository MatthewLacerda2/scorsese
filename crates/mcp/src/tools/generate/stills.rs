//! The stills half of the `generate` tool: Gemini, and what it costs.
//!
//! Shaped like [`lines`](super::lines): a picture comes back on the call —
//! unless it was ordered in a half-price batch (#894), which a later call or a
//! collect picks up.

use scorsese_core::{AssetId, Project};
use scorsese_providers::credentials::{Budget, Provider, resolve};
use scorsese_providers::image::{GeminiProvider, Outcome, Stopped, batch, collect, generate};

/// What one stills run produced.
pub(super) type Drawn = Vec<(AssetId, Outcome)>;

/// Draws every still that needs it, into `drawn`. The key — the Gemini key
/// Veo also takes — is resolved only when this pass has something to do.
///
/// **A run that stops part-way still fills `drawn`** with the stills it drew
/// before stopping, and the error is the whole story in words (#1153).
pub(super) fn pass(
    project: &mut Project,
    dir: &std::path::Path,
    budget: Budget,
    drawn: &mut Drawn,
) -> Result<(), String> {
    let key = resolve(Provider::Gemini).map_err(|error| super::stopped_at(&error))?;
    let provider = GeminiProvider::new(&key.secret);
    kept(generate(project, dir, &provider, budget), drawn)
}

/// Orders every still that needs it as a half-price batch, collecting any
/// earlier batch first — into `drawn`, as [`pass`] does.
pub(super) fn order(
    project: &mut Project,
    dir: &std::path::Path,
    budget: Budget,
    drawn: &mut Drawn,
) -> Result<(), String> {
    let key = resolve(Provider::Gemini).map_err(|error| format!("{error}"))?;
    let provider = GeminiProvider::new(&key.secret);
    kept(batch(project, dir, &provider, budget), drawn)
}

/// What a run did, into `drawn` — a stopped one's included — and why it
/// stopped, in words.
fn kept(run: Result<Drawn, Stopped>, drawn: &mut Drawn) -> Result<(), String> {
    match run {
        Ok(run) => *drawn = run,
        Err(stopped) => {
            let says = stopped.says();
            *drawn = stopped.run;
            return Err(says);
        }
    }
    Ok(())
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
