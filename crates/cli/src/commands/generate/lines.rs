//! The narration half of `scorsese generate`: ElevenLabs, and what it costs.
//!
//! The sibling of [`shots`](super::shots), and shorter for the reason the
//! provider is: nothing is ever in flight, so there is no sweep here and no
//! outcome that means *come back later*.

use std::path::Path;

use anyhow::Result;
use scorsese_core::{AssetId, AssetKind, Project};
use scorsese_providers::credentials::{Budget, Provider, resolve};
use scorsese_providers::prices::dollars;
use scorsese_providers::speech::{ElevenLabsProvider, Outcome, generate};

/// Speaks every line that needs it.
///
/// The key is resolved **here**, and only when this pass has something to do —
/// see [`super::wants`]. A project of nothing but Veo shots must never be
/// asked for an ElevenLabs key.
pub(super) fn pass(
    project: &mut Project,
    project_dir: &Path,
    budget: Budget,
) -> Result<Vec<(AssetId, Outcome)>> {
    let key = resolve(Provider::ElevenLabs)?;
    let provider = ElevenLabsProvider::new(&key.secret);
    Ok(generate(project, project_dir, &provider, budget)?)
}

/// What the narration in this project would cost to speak.
///
/// Priced from the text that is already in the document, so unlike a shot this
/// is exact before anything is sent — the vendor bills by character. It is
/// still an estimate: the rate is a page somebody copied, and library-voice
/// multipliers are deliberately ignored.
pub(super) fn quote(project: &Project) -> Result<u64> {
    let mut total = 0;
    for asset in project
        .assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::GeneratedAudio)
    {
        let request = asset.speech_request();
        let characters = asset.prompt.as_ref().map_or(0, |line| line.chars().count());
        let estimate = scorsese_providers::prices::speech(request.model, characters)?;
        println!(
            "{:<24} {} — {} characters in {}",
            asset.id,
            dollars(estimate.cents),
            estimate.characters,
            request.model.as_str(),
        );
        total += estimate.cents;
    }
    Ok(total)
}

/// One line per line of narration.
pub(super) fn report(outcomes: &[(AssetId, Outcome)]) {
    for (id, outcome) in outcomes {
        println!("{id:<24} {}", says(outcome));
    }
}

/// What one outcome reads as.
fn says(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Cached { path } => format!("already spoken — {path}"),
        Outcome::Generated { path, bytes, .. } => format!("spoken — {path} ({bytes} bytes)"),
        // Not a failure and not phrased as one: a line nobody has chosen a
        // voice for yet is a cut being written, and the run carried on.
        Outcome::Incomplete { why } => format!("not yet — {why}"),
        Outcome::Failed { message } => format!("refused — {message}"),
    }
}
