//! The ElevenLabs Music half of a pass: compose the tracks that need it
//! (#1264) — [`lines`](super::lines)' shape, since a track comes back on the
//! connection it was asked for too.

use std::path::Path;

use scorsese_core::{AssetId, Project};
use scorsese_providers::credentials::{Budget, Provider, resolve};
use scorsese_providers::music::{self, ElevenLabsComposer, Outcome, generate};

use super::{Pass, counted};

/// Composes every track that needs it, into `composed` — resolving the key
/// only when there is one, and filling `composed` even when the run stops.
pub(super) fn pass(
    project: &mut Project,
    root: &Path,
    pass: Pass,
    budget: Budget,
    composed: &mut Vec<(AssetId, Outcome)>,
) -> Result<(), String> {
    if pass != Pass::Submit || !music::pending(project, root) {
        return Ok(());
    }
    let key = resolve(Provider::ElevenLabs).map_err(|error| error.to_string())?;
    let provider = ElevenLabsComposer::new(&key.secret);
    match generate(project, root, &provider, budget) {
        Ok(run) => *composed = run,
        Err(stopped) => {
            let says = stopped.says();
            *composed = stopped.run;
            return Err(says);
        }
    }
    Ok(())
}

/// The tracks that arrived on disk on this pass, and so have something to
/// measure.
pub(super) fn arrived(composed: &[(AssetId, Outcome)]) -> impl Iterator<Item = &AssetId> {
    composed
        .iter()
        .filter(|(_, outcome)| matches!(outcome, Outcome::Generated { .. }))
        .map(|(id, _)| id)
}

/// What this pass is calculated to have spent, in US cents.
pub(super) fn spent(composed: &[(AssetId, Outcome)]) -> u64 {
    composed
        .iter()
        .map(|(_, outcome)| outcome.spent_cents())
        .sum()
}

/// What became of the tracks, as clauses for the one line the dialog shows.
pub(super) fn said(composed: &[(AssetId, Outcome)]) -> Vec<String> {
    let count = |wanted: fn(&Outcome) -> bool| composed.iter().filter(|(_, o)| wanted(o)).count();
    [
        counted(
            count(|outcome| matches!(outcome, Outcome::Generated { .. })),
            "composed",
        ),
        counted(
            count(|outcome| matches!(outcome, Outcome::Incomplete { .. })),
            "tracks not ready",
        ),
        counted(
            count(|outcome| matches!(outcome, Outcome::Failed { .. })),
            "tracks refused",
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}
