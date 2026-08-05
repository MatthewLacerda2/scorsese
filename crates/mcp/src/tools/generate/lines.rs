//! The narration half of the `generate` tool: ElevenLabs, and what it costs.
//!
//! The sibling of [`shots`](super::shots), and shorter for the reason the
//! provider is: nothing is ever in flight, so there is no collecting here and
//! no outcome that means *come back later*.

use scorsese_core::{AssetId, AssetKind, Project};
use scorsese_providers::credentials::{Budget, Provider, resolve};
use scorsese_providers::prices::{dollars, speech};
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

/// What the narration would cost, without a key and without sending anything.
///
/// Exact before anything is sent, because the vendor bills by character and the
/// characters are already in the document. Still an estimate: the rate is a
/// page somebody copied, and library-voice multipliers are deliberately
/// ignored.
pub(super) fn quote(project: &Project, lines: &mut Vec<String>) -> Result<u64, String> {
    let mut total = 0;
    for asset in project
        .assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::GeneratedAudio)
    {
        let request = asset.speech_request();
        let characters = asset.prompt.as_ref().map_or(0, |line| line.chars().count());
        let priced = speech(request.model, characters).map_err(|error| format!("{error}"))?;
        lines.push(format!(
            "{}: {} — {} characters in {}",
            asset.id,
            dollars(priced.cents),
            priced.characters,
            request.model.as_str()
        ));
        total += priced.cents;
    }
    Ok(total)
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
