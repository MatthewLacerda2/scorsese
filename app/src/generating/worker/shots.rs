//! The Veo half of a pass: hand the shots over, or ask after the ones already
//! going.
//!
//! The only half with a *later* in it. A Veo generation outlives the request
//! that started it, so this is the pass a sweep exists for — and the one whose
//! outcomes include two that mean neither made nor refused.

use std::path::Path;
use std::time::Duration;

use scorsese_core::{AssetId, Project};
use scorsese_providers::credentials::{Budget, Provider, resolve};
use scorsese_providers::video::{self, Outcome, Run, VeoProvider, collect, generate_waiting};

use super::{Pass, counted};

/// Submits or collects, and says nothing to the vendor when there is nothing to
/// say.
///
/// **The key is resolved here, and only when this pass has work.** A project
/// whose shots are all made and present asks Veo for nothing at all — which is
/// what makes a narration-only project generatable by somebody who has one
/// vendor's key and not the other's.
///
/// **A run that stops part-way still fills `made`** with the shots it sent,
/// and the error is the run in one sentence — so many sent, so many not —
/// rather than the bare reason (#1057).
pub(super) fn pass(
    project: &mut Project,
    root: &Path,
    pass: Pass,
    budget: Budget,
    made: &mut Run,
) -> Result<(), String> {
    if !video::pending(project, root) {
        return Ok(());
    }
    let key = resolve(Provider::Gemini).map_err(|error| error.to_string())?;
    let provider = VeoProvider::new(&key.secret);
    match pass {
        // Zero patience: the submit itself is what is wanted, and the window
        // does the waiting by sweeping.
        Pass::Submit => {
            match generate_waiting(project, root, &provider, budget, Duration::ZERO, |_| {}) {
                Ok(run) => *made = run,
                Err(stopped) => {
                    let says = stopped.says();
                    *made = stopped.run;
                    return Err(says);
                }
            }
        }
        Pass::Sweep => {
            made.outcomes = collect(project, root, &provider).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

/// The shots that arrived on disk on this pass, and so have something to
/// measure.
///
/// A cache hit is not one: its file was already there, and whatever ran when it
/// first landed has had every chance to look at it.
pub(super) fn arrived(outcomes: &[(AssetId, Outcome)]) -> impl Iterator<Item = &AssetId> {
    outcomes
        .iter()
        .filter(|(_, outcome)| matches!(outcome, Outcome::Generated { .. }))
        .map(|(id, _)| id)
}

/// What became of the shots, as clauses for the one line the dialog shows.
pub(super) fn said(outcomes: &[(AssetId, Outcome)]) -> Vec<String> {
    let count = |wanted: fn(&Outcome) -> bool| {
        outcomes
            .iter()
            .filter(|(_, outcome)| wanted(outcome))
            .count()
    };
    [
        counted(count(Outcome::is_in_flight), "generating"),
        counted(
            count(|outcome| matches!(outcome, Outcome::Generated { .. })),
            "arrived",
        ),
        counted(
            count(|outcome| matches!(outcome, Outcome::Failed { .. })),
            "refused",
        ),
        counted(
            count(|outcome| matches!(outcome, Outcome::Expired { .. })),
            "past the window and lost",
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}
