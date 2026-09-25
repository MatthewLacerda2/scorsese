//! The narration half of the `generate` tool: ElevenLabs, and what it costs.
//!
//! The sibling of [`shots`](super::shots), and shorter for the reason the
//! provider is: nothing is ever in flight, so there is no collecting here and
//! no outcome that means *come back later*.

use scorsese_core::{AssetId, Project};
use scorsese_providers::credentials::{Budget, Provider, resolve};
use scorsese_providers::speech::{ElevenLabsProvider, Outcome, generate};

/// What one narration run produced.
pub(super) type Spoken = Vec<(AssetId, Outcome)>;

/// Speaks every line that needs it.
///
/// The key is resolved **here**, and only when this pass has something to do —
/// a project of nothing but Veo shots must never be asked for an ElevenLabs
/// key.
pub(super) fn pass(
    project: &mut Project,
    dir: &std::path::Path,
    budget: Budget,
) -> Result<Spoken, String> {
    let key = resolve(Provider::ElevenLabs).map_err(|error| format!("{error}"))?;
    let provider = ElevenLabsProvider::new(&key.secret);
    generate(project, dir, &provider, budget).map_err(|error| format!("{error}"))
}

/// What the narration in a run reads as.
pub(super) fn said(spoken: &Spoken, lines: &mut Vec<String>) {
    for (id, outcome) in spoken {
        lines.push(format!("{id}: {}", one(outcome)));
    }
}

/// What one line's outcome reads as.
fn one(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Cached { path } => format!("already spoken — {path}"),
        Outcome::Generated { path, bytes, .. } => format!("spoken — {path} ({bytes} bytes)"),
        // Not a failure and not phrased as one: a line nobody has chosen a
        // voice for yet is a cut being written, and the run carried on.
        Outcome::Incomplete { why } => format!("not yet — {why}"),
        Outcome::Failed { message } => format!("refused — {message}"),
    }
}
