//! Measuring what a run brought back, and fitting the cut to it.
//!
//! **A generation has no measured shape until something looks**, and neither
//! kind of brief can supply one. How long a line takes depends on the words;
//! whether Veo put an engine note under a shot is not in the prompt either, and
//! it routinely does. `scorsese-providers` can answer neither question — it has
//! no decoder and must not grow one. The mix reads `duration_seconds` and
//! `audio_channels`, so a generation nobody probed is a line the mix skips and
//! a shot that plays silent. So the prober is found here, above it, and handed
//! to [`Done::measure`], which every local surface shares.
//!
//! **And a measured generation can be shorter than its clip.** A sketch is laid
//! out at whatever length its clip was given; a reading takes as long as its
//! words do. Saved as it stands, the next open refuses the project
//! (`ClipOutlastsSource`) and every command with it, so the clips of what
//! landed are shortened to it before the save and each one is said (#825,
//! #1114) — the same answer the MCP tool and the server give.

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::placing::Shortened;
use scorsese_core::{ProbeMedia, Project};
use scorsese_providers::passes::Done;
use scorsese_render::Ffprobe;

/// Probes everything unmeasured, shortens the clips of what `done` landed that
/// now outlast their source, and saves — answering with the clips it shortened.
pub(super) fn measure(
    project: &mut Project,
    project_dir: &Path,
    done: &Done,
) -> Result<Vec<Shortened>> {
    let probe = Ffprobe::discover().context("ffprobe is needed to measure what was generated")?;
    fitted(project, project_dir, &probe, done)
}

/// [`measure`] with the prober handed in, which is what lets it be tested
/// without a real file to decode.
fn fitted(
    project: &mut Project,
    project_dir: &Path,
    probe: &dyn ProbeMedia,
    done: &Done,
) -> Result<Vec<Shortened>> {
    let shortened = done.measure(project, project_dir, probe);
    project
        .save(project_dir)
        .with_context(|| format!("saving {}", project_dir.display()))?;
    Ok(shortened)
}

#[cfg(test)]
mod tests;
