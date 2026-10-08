//! A sheet of several pictures — one each from several files — rather than
//! several moments of one (#900).
//!
//! What a stock search shows: one small JPEG per result, tiled with its
//! number under it, so choosing between five shots costs one picture to look
//! at. The results are of mixed shapes, so each is fitted inside one cell
//! size and letterboxed rather than cropped — a preview that hides the edge of
//! a shot would mislead the choice it is for.

use std::path::{Path, PathBuf};

use scorsese_compositor::sheet::{self, Cell};
use scorsese_compositor::text::Font;
use scorsese_compositor::{Frame, Resolution};

use super::ContactError;
use super::sample::CELL_LONGEST_SIDE;
use crate::tools::Tools;

/// The cell's shorter side: a 16:9 cell on the longest one.
const CELL_SHORTER_SIDE: u32 = CELL_LONGEST_SIDE * 9 / 16;

/// Tiles each `(file, label)` into one sheet, at most
/// [`MAX_FRAMES`](super::MAX_FRAMES) of them — cells taller than wide when
/// `vertical`, so a vertical search is not shown as slivers.
pub fn pictures(
    tools: &Tools,
    pictures: &[(PathBuf, String)],
    vertical: bool,
) -> Result<Frame, ContactError> {
    let (width, height) = if vertical {
        (CELL_SHORTER_SIDE, CELL_LONGEST_SIDE)
    } else {
        (CELL_LONGEST_SIDE, CELL_SHORTER_SIDE)
    };
    let cell = Resolution::new(width, height).map_err(|source| ContactError::Cell {
        file: PathBuf::new(),
        source,
    })?;
    let mut cells = Vec::with_capacity(pictures.len());
    for (file, label) in pictures {
        cells.push(Cell {
            frame: fitted(tools, file, cell)?,
            label: label.clone(),
        });
    }
    Ok(sheet::tile(cells, Font::sans(), false)?)
}

/// `file`'s first frame, fitted inside `cell` and letterboxed in black.
fn fitted(tools: &Tools, file: &Path, cell: Resolution) -> Result<Frame, ContactError> {
    let (width, height) = (cell.width(), cell.height());
    let mut command = tools.ffmpeg();
    command
        .args(["-nostdin", "-v", "error", "-i"])
        .arg(file)
        .args(["-frames:v", "1"])
        .args([
            "-vf",
            &format!(
                "scale={width}:{height}:force_original_aspect_ratio=decrease,\
                 pad={width}:{height}:(ow-iw)/2:(oh-ih)/2:color=black,format=rgba"
            ),
        ])
        .arg("-an");
    crate::frames::raw_frame(command, file, cell).map_err(ContactError::from)
}
