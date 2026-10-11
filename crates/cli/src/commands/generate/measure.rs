//! Measuring what a run brought back, and fitting the cut to it.
//!
//! **A generation has no measured shape until something looks**, and neither
//! kind of brief can supply one. How long a line takes depends on the words;
//! whether Veo put an engine note under a shot is not in the prompt either, and
//! it routinely does. `scorsese-providers` can answer neither question — it has
//! no decoder and must not grow one. The mix reads `duration_seconds` and
//! `audio_channels`, so a generation nobody probed is a line the mix skips and
//! a shot that plays silent. This is where that gap is closed, above both
//! crates, which is where it belongs.
//!
//! **And a measured generation can be shorter than its clip.** A sketch is laid
//! out at whatever length its clip was given; a reading takes as long as its
//! words do. Saved as it stands, the next open refuses the project
//! (`ClipOutlastsSource`) and every command with it, so the clips of what
//! landed are shortened to it here and each one is said (#825, #1114) — the
//! same answer the MCP tool and the server give.

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::placing::{self, Shortened};
use scorsese_core::{AssetId, ProbeMedia, Project, Reprobe, probe_assets};
use scorsese_render::Ffprobe;

/// Probes everything unmeasured, shortens the clips of `landed` that now
/// outlast their source, and saves — answering with the clips it shortened.
pub(super) fn measure(
    project: &mut Project,
    project_dir: &Path,
    landed: &[AssetId],
) -> Result<Vec<Shortened>> {
    let probe = Ffprobe::discover().context("ffprobe is needed to measure what was generated")?;
    fitted(project, project_dir, &probe, landed)
}

/// [`measure`] with the prober handed in, which is what lets it be tested
/// without a real file to decode.
fn fitted(
    project: &mut Project,
    project_dir: &Path,
    probe: &dyn ProbeMedia,
    landed: &[AssetId],
) -> Result<Vec<Shortened>> {
    probe_assets(project, project_dir, probe, Reprobe::Skip);
    let shortened = placing::fit_to_sources(project, landed);
    project
        .save(project_dir)
        .with_context(|| format!("saving {}", project_dir.display()))?;
    Ok(shortened)
}

/// One line per clip shortened, after the run's own report.
pub(super) fn report(shortened: &[Shortened]) {
    for clip in shortened {
        println!("{}", clip.says());
    }
}

#[cfg(test)]
mod tests;
