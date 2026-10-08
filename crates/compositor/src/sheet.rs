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
//! **The label has a strip of its own, under the frame (#919).** It used to be
//! drawn over the bottom of each cell, which hid exactly the band a sheet is
//! most often taken to check — captions, lower thirds, a call to action — and
//! sent the reader back for a full still of every frame. Now every pixel of
//! each frame is on the sheet, and the sheet is taller than its cells by one
//! strip per row.
//!
//! Like [`card`], this is not a rendering path of its own. It
//! writes a label under each cell with the same [`card::draw`] a slug card
//! uses, then places the cells with the same [`Compositor`] a video frame goes
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

/// How tall a cell's label strip is, as a fraction of the cell's **shorter**
/// side.
///
/// The shorter side, so a 9:16 cell gets the same strip a 16:9 one of the same
/// pixels does rather than one sized off its height and set too wide for it.
const LABEL_HEIGHT: f64 = 0.13;

/// Em size of a label, as a fraction of the cell's shorter side — at most;
/// [`label_size`] sets it smaller when the longest label would not fit across.
const LABEL_SIZE: f64 = 0.085;

/// How much of a cell's width a label may take, leaving a margin either side.
const LABEL_SPAN: f64 = 0.94;

/// The strip a label is written on: a dark grey rather than black, so the
/// bottom edge of a frame that ends in black is still visible against it.
const LABEL_PANEL: Rgba = Rgba::new(0x1c, 0x1c, 0x1c, 0xff);

/// One frame of a sheet, and what to write on it.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// The picture. Every cell of a sheet must be the same size — they come
    /// from one file at one scale, so that costs the caller nothing.
    pub frame: Frame,
    /// What moment it is, e.g. `0:10`. Written in a strip under the cell,
    /// never over it.
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

/// Tiles `cells` into one picture, each labelled in a strip under it.
///
/// Every cell keeps all of its pixels: the strip is added below the frame, so
/// the sheet is `columns × width` across and `rows × (height + strip)` down.
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
pub fn tile(cells: Vec<Cell>, font: &Font, ruled: bool) -> Result<Frame, SheetError> {
    let picture = layout_size(&cells)?;
    let (columns, rows) = grid(cells.len());
    let strip = strip_height(picture);
    let size = label_size(&cells, picture, font);
    let cell = Resolution::new(picture.width(), picture.height() + strip).map_err(|source| {
        SheetError::Raster {
            columns,
            rows,
            cell: picture,
            source,
        }
    })?;
    let canvas_size =
        Resolution::new(columns * cell.width(), rows * cell.height()).map_err(|source| {
            SheetError::Raster {
                columns,
                rows,
                cell,
                source,
            }
        })?;

    let cells: Vec<Frame> = cells
        .into_iter()
        .map(|mut one| {
            // Over the picture alone: its fractions are the source's, and the
            // strip under it is no part of the frame they measure.
            if ruled {
                crate::grid::draw(&mut one.frame);
            }
            labelled(&one, cell, size, font)
        })
        .collect();

    let mut canvas = Frame::black(canvas_size);
    let layers: Vec<Layer<'_>> = cells
        .iter()
        .enumerate()
        .map(|(index, one)| Layer {
            source: one,
            properties: at(index, columns, cell, canvas_size),
            anchor: Anchor {
                x: AnchorX::Left,
                y: AnchorY::Top,
            },
            // A contact sheet places its cells and never turns them, so the
            // pivot is the one thing here that cannot matter.
            origin: scorsese_core::Origin::default(),
            matte: None,
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

/// How tall the label strip under a cell of `picture` is: even, so the
/// sheet stays a raster the encoder takes, and never nothing.
fn strip_height(picture: Resolution) -> u32 {
    let shorter = f64::from(picture.width().min(picture.height()));
    let strip = (shorter * LABEL_HEIGHT).round() as u32;
    (strip.max(2) / 2) * 2
}

/// The em size every label of the sheet is set at.
///
/// One size for the sheet, so the cells read as a row of the same thing: the
/// [`LABEL_SIZE`] share of the shorter side, made smaller only as far as the
/// longest label needs to fit across a cell. A tall, narrow cell would
/// otherwise cut `0:09.1 · frame 285` short — the frame number is the half a
/// following call names the instant by.
fn label_size(cells: &[Cell], picture: Resolution, font: &Font) -> f32 {
    let preferred = (f64::from(picture.width().min(picture.height())) * LABEL_SIZE) as f32;
    let span = (f64::from(picture.width()) * LABEL_SPAN) as f32;
    let widest = cells
        .iter()
        .map(|one| crate::text::width(font, &one.label, preferred))
        .fold(0.0_f32, f32::max);
    if widest > span {
        preferred * span / widest
    } else {
        preferred
    }
}

/// `one`'s picture with its moment written in a strip under it, as a frame of
/// `cell` — the picture's width, and its height plus the strip.
fn labelled(one: &Cell, cell: Resolution, size: f32, font: &Font) -> Frame {
    let mut framed = Frame::black(cell);
    let picture = one.frame.bytes();
    // Same width, so the picture's rows are the first rows of the cell.
    framed.bytes_mut()[..picture.len()].copy_from_slice(picture);
    let top = one.frame.resolution().height();
    let strip = cell.height() - top;
    card::draw(
        &mut framed,
        &Card {
            band: Band {
                top: top as f32,
                height: strip as f32,
            },
            background: LABEL_PANEL,
            text: &one.label,
            style: Style {
                figures: Default::default(),
                size,
                color: Rgba::WHITE,
                align: TextAlign::Center,
                line_height: strip as f32,
                max_width: cell.width() as f32,
                // A contact-sheet label sits on its own panel of colour, so
                // there is nothing behind it for a rim to rescue it from.
                edge: None,
                anchor: Anchor::default(),
            },
        },
        font,
    );
    framed
}
