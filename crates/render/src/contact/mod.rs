//! Contact sheets: several frames of a video file, tiled into one picture.
//!
//! The tool for **looking at footage that has not been edited yet**. Everything
//! else in this crate works on a project — a plan, a timeline, a render — and
//! answers questions about the cut. This answers a question about the material:
//! *what is actually in this file?* An assistant handed twenty clips can read
//! their durations, codecs and names, and from that alone it is arranging files
//! by their file names and hoping. A sheet is how it finds out that clip 3 is a
//! wide establishing shot and clip 7 is the same subject in close-up.
//!
//! It also closes the loop on generated video: Veo returns a clip that may or
//! may not be what the brief asked for, and without this nobody can tell. The
//! loop only finishes — generate, look, decide whether to re-prompt — if
//! looking is possible.
//!
//! **Any video file, project or not.** A sheet is taken of a path, so an
//! imported asset and a file somebody has not imported yet are the same call.
//! Nothing here loads a `project.json`.
//!
//! The division of labour is the usual one, unchanged. ffmpeg decodes the
//! frames, through [`Tools`] like every other invocation; the compositor tiles
//! them and draws the timestamps, because compositing is ours.

mod sample;

use std::path::{Path, PathBuf};

use scorsese_compositor::sheet::{self, Cell, SheetError};
use scorsese_compositor::text::Font;
use scorsese_core::probe::{ProbeError, ProbeMedia};

use crate::frames::FrameError;
use crate::probe::Ffprobe;
use crate::tools::Tools;
use scorsese_compositor::Frame;

pub use sample::{CELL_LONGEST_SIDE, Look, MAX_FRAMES, STEP_SECONDS, label};

/// A finished contact sheet, and what it is a sheet of.
///
/// The moments come back beside the picture rather than only drawn on it,
/// because a client that cannot see images still has to get a useful answer —
/// the same rule the other picture-returning tools follow. *"Frames at 0:00,
/// 0:05 and 0:10 of a 12-second file"* is worth having even blind.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    /// The tiled picture.
    pub image: Frame,
    /// The moments it shows, in seconds, in the order they are laid out.
    pub at_seconds: Vec<f64>,
    /// How long the whole file is, so a caller can tell how much it has seen.
    pub duration_seconds: f64,
    /// The file this is a sheet of.
    pub file: PathBuf,
}

impl Sheet {
    /// Where a following call should start to carry on through the file, or
    /// `None` when this sheet already reached the end.
    ///
    /// A file longer than one sheet is walked in successive calls, and this is
    /// the number that makes that a step rather than a guess.
    pub fn next_from(&self) -> Option<f64> {
        let last = self.at_seconds.last().copied()?;
        let next = last + STEP_SECONDS;
        (next < self.duration_seconds).then_some(next)
    }
}

/// Takes a contact sheet of `file`.
///
/// Probes it first, because how long the file is decides how many frames there
/// are to take and how big a cell is — and because a file with no video stream
/// has to be refused with that said, rather than by five decodes all failing
/// for reasons that do not name the problem.
pub fn sheet(tools: &Tools, file: &Path, look: &Look) -> Result<Sheet, ContactError> {
    let media = Ffprobe::new(tools.clone()).probe(file)?;
    let (Some(width), Some(height)) = (media.width, media.height) else {
        return Err(ContactError::NoPicture {
            file: file.to_path_buf(),
        });
    };
    let duration_seconds = media.duration_seconds.unwrap_or_default();

    let at_seconds = look.moments(duration_seconds);
    let (cell_width, cell_height) = sample::cell(width, height);
    let resolution =
        scorsese_compositor::Resolution::new(cell_width, cell_height).map_err(|source| {
            ContactError::Cell {
                file: file.to_path_buf(),
                source,
            }
        })?;

    let mut cells = Vec::with_capacity(at_seconds.len());
    for at in &at_seconds {
        cells.push(Cell {
            frame: decode(tools, file, *at, resolution)?,
            label: sample::label(*at),
        });
    }

    Ok(Sheet {
        image: sheet::tile(cells, Font::sans())?,
        at_seconds,
        duration_seconds,
        file: file.to_path_buf(),
    })
}

/// Decodes the frame at `at_seconds`, scaled to one cell.
///
/// `-ss` ahead of `-i` so ffmpeg seeks rather than decoding up to the moment:
/// on a long file that is the difference between a second and a minute, and the
/// keyframe-accurate seek it does instead is exactly right for a picture whose
/// whole purpose is *roughly what is on screen around here*.
fn decode(
    tools: &Tools,
    file: &Path,
    at_seconds: f64,
    resolution: scorsese_compositor::Resolution,
) -> Result<Frame, ContactError> {
    let mut command = tools.ffmpeg();
    command
        .args(["-nostdin", "-v", "error"])
        .arg("-ss")
        .arg(format!("{at_seconds:.3}"))
        .arg("-i")
        .arg(file)
        .args(["-frames:v", "1"])
        .args([
            "-vf",
            &format!(
                "scale={}:{},format=rgba",
                resolution.width(),
                resolution.height()
            ),
        ])
        .arg("-an");
    // The output — `-f rawvideo` and the `-` that names stdout — belongs to
    // `raw_frame` and is added there. Adding it here as well gave ffmpeg *two*
    // outputs, and `-frames:v 1` bounded only the first: the second wrote the
    // rest of the file down the same pipe.
    crate::frames::raw_frame(command, file, resolution).map_err(ContactError::from)
}

/// Why a sheet could not be taken.
#[derive(Debug, thiserror::Error)]
pub enum ContactError {
    /// ffprobe could not read the file at all.
    #[error(transparent)]
    Probe(#[from] ProbeError),
    /// The file opened, and there is no picture in it — an audio file, or a
    /// container whose video stream is missing. Named separately because
    /// "there is nothing to look at" is a different answer from "this is
    /// broken".
    #[error("{} has no video stream, so there is nothing to look at", file.display())]
    NoPicture {
        /// The file that was probed.
        file: PathBuf,
    },
    /// The source's shape does not scale to a usable cell.
    #[error("{} does not scale to a cell of any usable size: {source}", file.display())]
    Cell {
        /// The file being sampled.
        file: PathBuf,
        /// What the resolution refused.
        #[source]
        source: scorsese_compositor::ResolutionError,
    },
    /// A frame would not decode.
    #[error(transparent)]
    Frame(#[from] FrameError),
    /// The cells would not tile.
    #[error(transparent)]
    Tile(#[from] SheetError),
}
