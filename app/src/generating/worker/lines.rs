//! The ElevenLabs half of a pass: speak the lines that need it.
//!
//! Shorter than [`shots`](super::shots) for the reason the provider is
//! shorter. A reading comes back on the connection it was asked for, so there
//! is no ticket, nothing in flight, and nothing for a sweep to collect — which
//! is why this pass runs on a submit and never otherwise.
//!
//! Measuring what came back is [`super`]'s, for both vendors at once — see
//! [`super::measure`].

use std::path::Path;

use scorsese_core::{AssetId, Project};
use scorsese_providers::credentials::{Budget, Provider, resolve};
use scorsese_providers::speech::{self, ElevenLabsProvider, Outcome, generate};

use super::{Pass, counted};

/// Speaks every line that needs it, into `spoken`.
///
/// **The key is resolved here, and only when this pass has work** — the same
/// rule the shots pass follows, and for the same reason: a project of nothing
/// but Veo shots must never be stopped for want of an ElevenLabs key.
///
/// **A run that stops part-way still fills `spoken`** with the lines it spoke,
/// and the error says in one sentence how many went (#1153).
pub(super) fn pass(
    project: &mut Project,
    root: &Path,
    pass: Pass,
    budget: Budget,
    spoken: &mut Vec<(AssetId, Outcome)>,
) -> Result<(), String> {
    if pass != Pass::Submit || !speech::pending(project, root) {
        return Ok(());
    }
    let key = resolve(Provider::ElevenLabs).map_err(|error| error.to_string())?;
    let provider = ElevenLabsProvider::new(&key.secret);
    match generate(project, root, &provider, budget) {
        Ok(run) => *spoken = run,
        Err(stopped) => {
            let says = stopped.says();
            *spoken = stopped.run;
            return Err(says);
        }
    }
    Ok(())
}

/// The lines that arrived on disk on this pass, and so have something to
/// measure.
pub(super) fn arrived(spoken: &[(AssetId, Outcome)]) -> impl Iterator<Item = &AssetId> {
    spoken
        .iter()
        .filter(|(_, outcome)| matches!(outcome, Outcome::Generated { .. }))
        .map(|(id, _)| id)
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
