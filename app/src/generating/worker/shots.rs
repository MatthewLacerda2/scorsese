//! The Veo half of a pass: hand the shots over, or ask after the ones already
//! going.
//!
//! The only half with a *later* in it. A Veo generation outlives the request
//! that started it, so this is the pass a sweep exists for — and the one whose
//! outcomes include two that mean neither made nor refused.

use std::path::Path;
use std::time::Duration;

use scorsese_core::{AssetId, AssetKind, Project};
use scorsese_providers::credentials::{Budget, Provider, resolve};
use scorsese_providers::video::{Outcome, Run, VeoProvider, collect, generate_waiting};

use super::{Pass, counted, wants};

/// Submits or collects, and says nothing to the vendor when there is nothing to
/// say.
///
/// **The key is resolved here, and only when this pass has work.** A project
/// whose shots are all made and present asks Veo for nothing at all — which is
/// what makes a narration-only project generatable by somebody who has one
/// vendor's key and not the other's.
pub(super) fn pass(
    project: &mut Project,
    root: &Path,
    pass: Pass,
    budget: Budget,
) -> Result<Run, String> {
    if !wants(project, root, AssetKind::GeneratedVideo) {
        return Ok(Run {
            outcomes: Vec::new(),
            spent_cents: 0,
        });
    }
    let key = resolve(Provider::Gemini).map_err(|error| error.to_string())?;
    let provider = VeoProvider::new(&key.secret);
    match pass {
        // Zero patience: the submit itself is what is wanted, and the window
        // does the waiting by sweeping.
        Pass::Submit => generate_waiting(project, root, &provider, budget, Duration::ZERO, |_| {})
            .map_err(|error| error.to_string()),
        Pass::Sweep => collect(project, root, &provider)
            .map(|outcomes| Run {
                outcomes,
                spent_cents: 0,
            })
            .map_err(|error| error.to_string()),
    }
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
