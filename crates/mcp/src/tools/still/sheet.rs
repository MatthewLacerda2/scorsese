//! `still` with `sheet: true`: the instants as one contact sheet (#814).
//!
//! The tiling and labels are `look`'s, through
//! [`scorsese_render::contact::timeline_sheet`]; what is here is only the
//! reply — one picture, and one sentence naming what is in it.

use std::path::Path;

use scorsese_core::{Frames, Project};
use scorsese_render::{Renderer, Resolution, Tools, contact};

use crate::tools::{Part, Reply};

/// Everything the sheet is drawn from, already parsed and checked.
pub(super) struct Asked<'a> {
    pub(super) tools: &'a Tools,
    pub(super) renderer: &'a Renderer<'a>,
    pub(super) project: &'a Project,
    pub(super) dir: &'a Path,
    pub(super) instants: &'a [Frames],
    /// Each cell's raster.
    pub(super) resolution: Resolution,
    pub(super) ruled: bool,
    /// The caller's `out`, as given and as resolved.
    pub(super) kept: Option<(&'a str, &'a Path)>,
}

/// Composites the instants, tiles them and answers with the one picture.
///
/// The notes still go in the text, each keyed by the instant it was noticed
/// at: on a sheet there is no "under the frame" to put them, so the sentence
/// has to say which cell a note is about. A note repeated at a later instant
/// is said once, at the first, as a run of separate pictures does.
pub(super) fn reply(asked: Asked<'_>) -> Result<Reply, String> {
    let fps = asked.project.timeline_fps;
    let drawn = contact::timeline_sheet(
        asked.renderer,
        asked.project,
        asked.dir,
        asked.instants,
        asked.ruled,
    )
    .map_err(|error| format!("{error}"))?;
    let bytes = super::png(asked.tools, asked.kept.map(|(_, path)| path), &drawn.image)?;

    let cells: Vec<String> = asked
        .instants
        .iter()
        .map(|at| format!("frame {} ({:.2}s)", at.get(), fps.seconds(*at)))
        .collect();
    let ruler = if asked.ruled {
        ", each ruled 0.0 to 1.0"
    } else {
        ""
    };
    let mut said = format!(
        "contact sheet of {} at {} cells{ruler}, left to right then down: {}",
        asked.project.name,
        asked.resolution,
        cells.join(", ")
    );
    if let Some((given, _)) = asked.kept {
        said.push_str(&format!(" — written to {given}"));
    }
    let mut told = Vec::new();
    for (at, notes) in drawn.notes {
        for note in notes {
            if !told.contains(&note) {
                said.push_str(&format!("\nnote at frame {}: {note}", at.get()));
                told.push(note);
            }
        }
    }
    Ok(vec![Part::picture(said, &bytes)].into())
}
