//! The stills half of a background pass: Gemini, one call per still.
//!
//! [`lines`](super::lines)' shape: nothing is ever in flight, so a sweep has
//! nothing to do here and only a submit draws.

use std::path::Path;

use scorsese_core::{AssetId, Project};
use scorsese_providers::credentials::{Budget, Provider, resolve};
use scorsese_providers::image::{self, GeminiProvider, Outcome, generate};

use super::{Pass, counted};

/// Draws every still that needs it, into `drawn`. The key — the Gemini key
/// Veo takes too — is resolved only when this pass has work.
///
/// **A run that stops part-way still fills `drawn`** with the stills it drew,
/// and the error says in one sentence how many went (#1153).
pub(super) fn pass(
    project: &mut Project,
    root: &Path,
    pass: Pass,
    budget: Budget,
    drawn: &mut Vec<(AssetId, Outcome)>,
) -> Result<(), String> {
    if pass != Pass::Submit || !image::pending(project, root) {
        return Ok(());
    }
    let key = resolve(Provider::Gemini).map_err(|error| error.to_string())?;
    let provider = GeminiProvider::new(&key.secret);
    match generate(project, root, &provider, budget) {
        Ok(run) => *drawn = run,
        Err(stopped) => {
            let says = stopped.says();
            *drawn = stopped.run;
            return Err(says);
        }
    }
    Ok(())
}

/// Whether a still arrived on disk on this pass, and so has something to measure.
pub(super) fn arrived(drawn: &[(AssetId, Outcome)]) -> bool {
    drawn.iter().any(|(_, outcome)| outcome.landed())
}

/// What this pass is calculated to have spent, in US cents.
pub(super) fn spent(drawn: &[(AssetId, Outcome)]) -> u64 {
    drawn.iter().map(|(_, outcome)| outcome.spent_cents()).sum()
}

/// What became of the stills, as clauses for the one line the dialog shows.
pub(super) fn said(drawn: &[(AssetId, Outcome)]) -> Vec<String> {
    let count = |wanted: fn(&Outcome) -> bool| drawn.iter().filter(|(_, o)| wanted(o)).count();
    [
        counted(
            count(|outcome| matches!(outcome, Outcome::Generated { .. })),
            "drawn",
        ),
        counted(
            count(|outcome| matches!(outcome, Outcome::Collected { .. })),
            "collected from a batch",
        ),
        counted(
            count(|outcome| matches!(outcome, Outcome::Waiting { .. })),
            "still in a batch",
        ),
        counted(
            count(|outcome| matches!(outcome, Outcome::Incomplete { .. })),
            "stills not ready",
        ),
        counted(
            count(|outcome| matches!(outcome, Outcome::Failed { .. })),
            "stills refused",
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}
