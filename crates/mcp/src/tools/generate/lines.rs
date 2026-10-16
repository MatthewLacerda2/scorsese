//! The narration half of the `generate` tool: ElevenLabs, and what it costs.
//!
//! The sibling of [`shots`](super::shots), and shorter for the reason the
//! provider is: nothing is ever in flight, so there is no collecting here and
//! no outcome that means *come back later*.

use scorsese_core::{AssetId, Project};
use scorsese_providers::credentials::{Budget, Provider, resolve};
use scorsese_providers::speech::{ElevenLabsProvider, Outcome, generate};

/// What one narration run produced.
pub(super) type Spoken = Vec<(AssetId, Outcome)>;

/// Speaks every line that needs it, into `spoken`.
///
/// The key is resolved **here**, and only when this pass has something to do —
/// a project of nothing but Veo shots must never be asked for an ElevenLabs
/// key. **A run that stops part-way still fills `spoken`** with the lines it
/// spoke before stopping, and the error is the whole story in words (#1153).
pub(super) fn pass(
    project: &mut Project,
    dir: &std::path::Path,
    budget: Budget,
    spoken: &mut Spoken,
) -> Result<(), String> {
    let key = resolve(Provider::ElevenLabs).map_err(|error| super::stopped_at(&error))?;
    let provider = ElevenLabsProvider::new(&key.secret);
    match generate(project, dir, &provider, budget) {
        Ok(run) => *spoken = run,
        Err(stopped) => {
            let says = stopped.says();
            *spoken = stopped.run;
            return Err(says);
        }
    }
    Ok(())
}

/// What the narration in a run reads as.
pub(super) fn said(spoken: &Spoken, dir: &std::path::Path, lines: &mut Vec<String>) {
    for (id, outcome) in spoken {
        lines.push(format!("{id}: {}", outcome.says_in(dir)));
    }
}
