//! Contact sheets of the **edit**: several instants of the timeline, each
//! composited, tiled into one picture with its moment written on it.
//!
//! The counterpart of [`super::sheet`], which looks at a file. The tiling and
//! the labels are the same code — [`sheet::tile`] — so a sheet of the cut and a
//! sheet of the footage read the same way; only where the cells come from
//! differs. Here each cell is [`Renderer::still_noted`], the render pipeline
//! with the encoder taken out, at the cell's own raster.
//!
//! **Why a sheet at all, when `still` already takes a list (#814).** Several
//! instants as several pictures cost an image apiece over MCP, the most
//! expensive thing the server sends, and on the command line they are several
//! PNGs somebody has to tile by hand before the frames can be compared. One
//! picture of five small cells is cheaper than one full frame and shows the
//! change between them, which is what a preview is looking for.

use std::path::Path;

use scorsese_compositor::sheet::{self, Cell, MAX_CELLS, SheetError};
use scorsese_compositor::text::Font;
use scorsese_core::{Frames, Project};

use crate::report::Note;
use crate::run::Renderer;
use crate::{Frame, RenderError};

use super::sample::label;

/// A finished sheet of the timeline, and what drawing each cell noticed.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineSheet {
    /// The tiled picture.
    pub image: Frame,
    /// What each instant's compositing noticed that the picture cannot say —
    /// a page that would not capture, say — in the order the cells are laid
    /// out. Kept apart per instant rather than merged, because the notes go in
    /// the text beside the picture and have to say which cell they are about.
    pub notes: Vec<(Frames, Vec<Note>)>,
}

/// Composites each of `at` with `renderer` and tiles the frames into one
/// picture, in the order given.
///
/// Each cell is composited at the renderer's raster, so that raster is the
/// **cell's** size and the sheet is a multiple of it. Every cell is labelled
/// with its time and its timeline frame, and `ruled` draws the coordinate grid
/// over each cell — a cell is a whole frame of the edit, so its fractions are
/// the ones `transform.position` is written in.
///
/// More instants than a sheet holds is refused before anything is composited:
/// five composites thrown away for a sixth that never fit is the slow way to
/// learn the cap.
pub fn timeline_sheet(
    renderer: &Renderer<'_>,
    project: &Project,
    project_root: &Path,
    at: &[Frames],
    ruled: bool,
) -> Result<TimelineSheet, TimelineSheetError> {
    if at.len() > MAX_CELLS {
        return Err(TimelineSheetError::TooMany { found: at.len() });
    }
    let fps = project.timeline_fps;
    let mut cells = Vec::with_capacity(at.len());
    let mut notes = Vec::with_capacity(at.len());
    for &instant in at {
        let (frame, noticed) = renderer
            .still_noted(project, project_root, instant)
            .map_err(|source| TimelineSheetError::Composite {
                at: instant,
                source,
            })?;
        cells.push(Cell {
            frame,
            label: cell_label(fps.seconds(instant), instant),
        });
        notes.push((instant, noticed));
    }
    Ok(TimelineSheet {
        image: sheet::tile(cells, Font::sans(), ruled)?,
        notes,
    })
}

/// What a cell of the edit is labelled: `0:09.1 · frame 285`.
///
/// The time as [`label`] writes it for a file, so the two sheets read alike,
/// and the timeline frame beside it, because that is the number a following
/// call or an edit names the instant by — a time rounded to a tenth can sit
/// between two frames.
pub fn cell_label(seconds: f64, frame: Frames) -> String {
    format!("{} · frame {}", label(seconds), frame.get())
}

/// Why a sheet of the timeline could not be made.
#[derive(Debug, thiserror::Error)]
pub enum TimelineSheetError {
    /// More instants than one sheet holds.
    #[error(
        "a sheet holds at most {MAX_CELLS} instants and {found} were asked for — \
         ask for {MAX_CELLS} or fewer, and call again for the rest"
    )]
    TooMany {
        /// How many were asked for.
        found: usize,
    },
    /// One instant would not composite.
    #[error("compositing frame {}: {source}", at.get())]
    Composite {
        /// The instant that failed.
        at: Frames,
        /// Why.
        #[source]
        source: RenderError,
    },
    /// The cells would not tile.
    #[error(transparent)]
    Tile(#[from] SheetError),
}
