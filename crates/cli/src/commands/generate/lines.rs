//! The narration half of `scorsese generate`: ElevenLabs, and what it costs.
//!
//! The sibling of [`shots`](super::shots), and shorter for the reason the
//! provider is: nothing is ever in flight, so there is no sweep here and no
//! outcome that means *come back later*.

use std::path::Path;

use anyhow::Result;
use scorsese_core::{AssetId, Project};
use scorsese_providers::credentials::{Budget, Provider, resolve};
use scorsese_providers::speech::{ElevenLabsProvider, Outcome, generate};

/// Speaks every line that needs it, into `spoken`.
///
/// The key is resolved **here**, and only when this pass has something to do —
/// see [`super::pending`]. A project of nothing but Veo shots must never be
/// asked for an ElevenLabs key. **A run that stops part-way still fills
/// `spoken`** with the lines it spoke before stopping, and the error says in
/// words how many went (#1153).
pub(super) fn pass(
    project: &mut Project,
    project_dir: &Path,
    budget: Budget,
    spoken: &mut Vec<(AssetId, Outcome)>,
) -> Result<()> {
    let key = resolve(Provider::ElevenLabs)?;
    let provider = ElevenLabsProvider::new(&key.secret);
    match generate(project, project_dir, &provider, budget) {
        Ok(run) => *spoken = run,
        Err(stopped) => {
            let says = stopped.says();
            *spoken = stopped.run;
            return Err(anyhow::anyhow!(says));
        }
    }
    Ok(())
}

/// One line per line of narration.
pub(super) fn report(outcomes: &[(AssetId, Outcome)], project_dir: &Path) {
    for (id, outcome) in outcomes {
        println!("{id:<24} {}", outcome.says_in(project_dir));
    }
}
