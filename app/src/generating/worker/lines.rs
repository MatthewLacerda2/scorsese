//! The ElevenLabs half of a pass: speak the lines that need it.
//!
//! Shorter than [`shots`](super::shots) for the reason the provider is
//! shorter. A reading comes back on the connection it was asked for, so there
//! is no ticket, nothing in flight, and nothing for a sweep to collect — which
//! is why this pass runs on a submit and never otherwise.
//!
//! The one thing it adds that the video pass does not need is the measuring.

use std::path::Path;

use scorsese_core::{AssetId, AssetKind, Project, Reprobe, probe_assets};
use scorsese_providers::credentials::{Budget, Provider, resolve};
use scorsese_providers::speech::{ElevenLabsProvider, Outcome, generate};
use scorsese_render::Ffprobe;

use super::{Pass, counted, wants};

/// Speaks every line that needs it, and measures what came back.
///
/// **The key is resolved here, and only when this pass has work** — the same
/// rule the shots pass follows, and for the same reason: a project of nothing
/// but Veo shots must never be stopped for want of an ElevenLabs key.
pub(super) fn pass(
    project: &mut Project,
    root: &Path,
    pass: Pass,
    budget: Budget,
) -> Result<Vec<(AssetId, Outcome)>, String> {
    if pass != Pass::Submit || !wants(project, root, AssetKind::GeneratedAudio) {
        return Ok(Vec::new());
    }
    let key = resolve(Provider::ElevenLabs).map_err(|error| error.to_string())?;
    let provider = ElevenLabsProvider::new(&key.secret);
    let spoken = generate(project, root, &provider, budget).map_err(|error| error.to_string())?;
    if spoken
        .iter()
        .any(|(_, outcome)| matches!(outcome, Outcome::Generated { .. }))
    {
        measure(project, root);
    }
    Ok(spoken)
}

/// What this pass is calculated to have spent, in US cents.
pub(super) fn spent(spoken: &[(AssetId, Outcome)]) -> u64 {
    spoken
        .iter()
        .map(|(_, outcome)| outcome.spent_cents())
        .sum()
}

/// What became of the lines, as clauses for the one line the dialog shows.
///
/// A line nobody has finished writing is counted as *not ready* rather than as
/// a failure, because that is what it is: a cut being written normally has one
/// in it, and the run carried on around it.
pub(super) fn said(spoken: &[(AssetId, Outcome)]) -> Vec<String> {
    let count = |wanted: fn(&Outcome) -> bool| spoken.iter().filter(|(_, o)| wanted(o)).count();
    [
        counted(
            count(|outcome| matches!(outcome, Outcome::Generated { .. })),
            "spoken",
        ),
        counted(
            count(|outcome| matches!(outcome, Outcome::Incomplete { .. })),
            "lines not ready",
        ),
        counted(
            count(|outcome| matches!(outcome, Outcome::Failed { .. })),
            "lines refused",
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Measures what was just spoken.
///
/// **Narration has no length until something looks.** A shot comes back exactly
/// as long as its request asked, so the provider fills that in from the brief;
/// how long a line takes depends on the words, and `scorsese-providers` has no
/// decoder to measure an MP3 with — nor should it grow one. The mix reads
/// `duration_seconds`, so an unprobed line is a line the mix skips.
///
/// A failure here is not reported and not an error. The audio is on disk and
/// has been paid for; a machine with no ffprobe should not be told its run
/// failed, and both `scorsese probe` and this window's own open-time pass are
/// still there to close the gap.
fn measure(project: &mut Project, root: &Path) {
    let Ok(probe) = Ffprobe::discover() else {
        return;
    };
    probe_assets(project, root, &probe, Reprobe::Skip);
}
