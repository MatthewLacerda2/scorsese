//! A contact sheet: several frames tiled into one picture, each stamped with
//! the moment it came from.
//!
//! **One image, not several.** An assistant reading five separate pictures pays
//! five times the attachment cost for less than it gets here, because what says
//! what a shot *does* is the change between the frames — and a change is only
//! visible when the frames are side by side. The label is what makes that
//! readable: without it a sheet is five pictures in a row, and with it the
//! reader knows the third one is nine seconds in.
//!
//! Like [`card`], this is not a rendering path of its own. It
//! stamps a label onto each cell with the same [`card::draw`] a slug card uses,
//! then places the cells with the same [`Compositor`] a video frame goes
//! through. What comes out is an ordinary [`Frame`].

use scorsese_core::{Anchor, AnchorX, AnchorY, Rgba, TextAlign};

use crate::card::{self, Card};
use crate::compose::{CompositeError, Compositor, Layer};
use crate::cpu::CpuCompositor;
use crate::frame::{Frame, Resolution, ResolutionError};
use crate::properties::Properties;
use crate::text::{Band, Font, Style};

/// The most cells a sheet may hold.
///
/// Five, and enforced here rather than trusted from a caller — the cap is what
/// bounds the cost of looking, and a limit that lives only in an argument's
/// documentation is a limit somebody eventually passes six to.
pub const MAX_CELLS: usize = 5;

/// How tall a cell's label strip is, as a fraction of the cell.
const LABEL_HEIGHT: f64 = 0.13;

/// Em size of a label, as a fraction of the cell's height.
const LABEL_SIZE: f64 = 0.085;

/// The strip behind a label: black, but not quite — the frame under it still
/// reads through, which keeps a timestamp from hiding the bottom of a shot.
const LABEL_PANEL: Rgba = Rgba::new(0x00, 0x00, 0x00, 0xb4);

/// One frame of a sheet, and what to write on it.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// The picture. Every cell of a sheet must be the same size — they come
    /// from one file at one scale, so that costs the caller nothing.
    pub frame: Frame,
    /// What moment it is, e.g. `0:10`. Drawn along the bottom of the cell.
    pub label: String,
}

/// Why a sheet could not be made.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SheetError {
    /// No cells at all — a file that yielded no frame to look at.
    #[error("a contact sheet of no frames is not a picture of anything")]
    Empty,
    /// More cells than [`MAX_CELLS`].
    #[error("a contact sheet holds at most {MAX_CELLS} frames, and {found} were given")]
    TooMany {
        /// How many arrived.
        found: usize,
    },
    /// Cells of differing sizes, which no grid can be laid out from.
    #[error("cell {index} is {found} but the first is {first} — a sheet's cells are one size")]
    Ragged {
        /// Which cell disagreed.
        index: usize,
        /// The size it is.
        found: Resolution,
        /// The size the sheet is being laid out at.
        first: Resolution,
    },
    /// The grid the cells imply is not a size a frame can be.
    #[error("a {columns}×{rows} grid of {cell} cells is not a usable raster: {source}")]
    Raster {
        /// Columns in the grid.
        columns: u32,
        /// Rows in the grid.
        rows: u32,
        /// The cell size being tiled.
        cell: Resolution,
        /// What the resolution refused.
        #[source]
        source: ResolutionError,
    },
    /// The compositor refused a layer.
    #[error("tiling the sheet: {0}")]
    Composite(#[from] CompositeError),
}

/// Tiles `cells` into one picture, each labelled along its bottom edge.
///
/// The arrangement is at most three across: a single row while that fits, two
/// rows beyond it. Wider would make a sheet a letterbox strip whose cells are
/// unreadable at any sensible width, and taller would put the last frame so
/// far from the first that the eye stops comparing them.
///
/// `ruled` draws [`crate::grid`]'s coordinates over **each cell**, which is the
/// only place they can go: a cell is one whole source frame, so its fractions
/// are the source's own — which is exactly what a `crop` is written in. Ruling
/// the finished sheet would measure the tiling instead.
pub fn tile(mut cells: Vec<Cell>, font: &Font, ruled: bool) -> Result<Frame, SheetError> {
    let cell = layout_size(&cells)?;
    let (columns, rows) = grid(cells.len());
    let canvas_size =
        Resolution::new(columns * cell.width(), rows * cell.height()).map_err(|source| {
            SheetError::Raster {
                columns,
                rows,
                cell,
                source,
            }
        })?;

    for one in &mut cells {
        stamp(one, font);
        // After the timestamp, not before: the strip that carries it is a
        // panel of near-black, and a `1.0` drawn under it would be the one
        // coordinate on the picture nobody could read.
        if ruled {
            crate::grid::draw(&mut one.frame);
        }
    }

    let mut canvas = Frame::black(canvas_size);
    let layers: Vec<Layer<'_>> = cells
        .iter()
        .enumerate()
        .map(|(index, one)| Layer {
            source: &one.frame,
            properties: at(index, columns, cell, canvas_size),
            anchor: Anchor {
                x: AnchorX::Left,
                y: AnchorY::Top,
            },
        })
        .collect();
    CpuCompositor::new().composite(&mut canvas, &layers)?;
    Ok(canvas)
}

/// The size every cell has to be, having checked that every cell is it.
fn layout_size(cells: &[Cell]) -> Result<Resolution, SheetError> {
    let first = cells.first().ok_or(SheetError::Empty)?.frame.resolution();
    if cells.len() > MAX_CELLS {
        return Err(SheetError::TooMany { found: cells.len() });
    }
    for (index, one) in cells.iter().enumerate() {
        let found = one.frame.resolution();
        if found != first {
            return Err(SheetError::Ragged {
                index,
                found,
                first,
            });
        }
    }
    Ok(first)
}

/// Columns and rows for this many cells.
///
/// One row up to three, then two. Five — the cap, and the default — is three
/// over two, which reads left to right and then down, the way the frames run.
const fn grid(count: usize) -> (u32, u32) {
    match count {
        0 | 1 => (1, 1),
        2 => (2, 1),
        3 => (3, 1),
        4 => (2, 2),
        _ => (3, 2),
    }
}

/// Where cell `index` sits, as the fractional offset a [`Layer`] takes.
///
/// Fractions of the canvas rather than pixels because that is the vocabulary
/// [`Properties::position`] speaks everywhere else — a layer nudged in pixels
/// would land somewhere else the moment the raster changed size.
fn at(index: usize, columns: u32, cell: Resolution, canvas: Resolution) -> Properties {
    let column = index as u32 % columns;
    let row = index as u32 / columns;
    Properties {
        position: (
            f64::from(column * cell.width()) / f64::from(canvas.width()),
            f64::from(row * cell.height()) / f64::from(canvas.height()),
        ),
        ..Properties::default()
    }
}

/// Writes a cell's moment along its bottom edge.
fn stamp(cell: &mut Cell, font: &Font) {
    let resolution = cell.frame.resolution();
    let height = f64::from(resolution.height());
    let strip = height * LABEL_HEIGHT;
    card::draw(
        &mut cell.frame,
        &Card {
            band: Band {
                top: (height - strip) as f32,
                height: strip as f32,
            },
            background: LABEL_PANEL,
            text: &cell.label,
            style: Style {
                size: (height * LABEL_SIZE) as f32,
                color: Rgba::WHITE,
                align: TextAlign::Center,
                line_height: strip as f32,
                max_width: resolution.width() as f32,
                anchor: Anchor::default(),
            },
        },
        font,
    );
}
