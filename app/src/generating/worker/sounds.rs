//! The ElevenLabs sound-effects half of a pass: make the sounds that need it
//! (#1265) — [`tracks`](super::tracks)' shape, since a sound comes back on the
//! connection it was asked for too.

use std::path::Path;

use scorsese_core::{AssetId, Project};
use scorsese_providers::credentials::{Budget, Provider, resolve};
use scorsese_providers::sound::{self, ElevenLabsSounds, Outcome, generate};

use super::{Pass, counted};

/// Makes every sound that needs it, into `made` — resolving the key only when
/// there is one, and filling `made` even when the run stops.
pub(super) fn pass(
    project: &mut Project,
    root: &Path,
    pass: Pass,
    budget: Budget,
    made: &mut Vec<(AssetId, Outcome)>,
) -> Result<(), String> {
    if pass != Pass::Submit || !sound::pending(project, root) {
        return Ok(());
    }
    let key = resolve(Provider::ElevenLabs).map_err(|error| error.to_string())?;
    let provider = ElevenLabsSounds::new(&key.secret);
    match generate(project, root, &provider, budget) {
        Ok(run) => *made = run,
        Err(stopped) => {
            let says = stopped.says();
            *made = stopped.run;
            return Err(says);
        }
    }
    Ok(())
}

/// The sounds that arrived on disk on this pass, and so have something to
/// measure.
pub(super) fn arrived(made: &[(AssetId, Outcome)]) -> impl Iterator<Item = &AssetId> {
    made.iter()
        .filter(|(_, outcome)| matches!(outcome, Outcome::Generated { .. }))
        .map(|(id, _)| id)
}

/// What this pass is calculated to have spent, in US cents.
pub(super) fn spent(made: &[(AssetId, Outcome)]) -> u64 {
    made.iter().map(|(_, outcome)| outcome.spent_cents()).sum()
}

/// What became of the sounds, as clauses for the one line the dialog shows.
pub(super) fn said(made: &[(AssetId, Outcome)]) -> Vec<String> {
    let count = |wanted: fn(&Outcome) -> bool| made.iter().filter(|(_, o)| wanted(o)).count();
    [
        counted(
            count(|outcome| matches!(outcome, Outcome::Generated { .. })),
            "sounds made",
        ),
        counted(
            count(|outcome| matches!(outcome, Outcome::Incomplete { .. })),
            "sounds not ready",
        ),
        counted(
            count(|outcome| matches!(outcome, Outcome::Failed { .. })),
            "sounds refused",
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}
