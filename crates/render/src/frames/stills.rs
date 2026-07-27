//! Pulling a handful of frames out of a finished render, as files on disk.
//!
//! The frames are chosen, never sampled: an agent asks for the ones it wants to
//! look at, and the useful default is every segment boundary, because a
//! boundary is where a cut can be one frame wrong. That is the same reasoning
//! the golden fixtures use when deciding which frames to compare, and it holds
//! for the same reason — a still from the middle of a long shot shows almost
//! nothing that the second before it did not.

use std::path::{Path, PathBuf};

use crate::settings::Resolution;
use crate::tools::Tools;

use super::{FrameError, extract, write_png};

/// One frame of a render, written out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Still {
    /// Which frame of the delivered file it is, counted from zero.
    pub frame: u64,
    /// Where the PNG was written.
    pub path: PathBuf,
}

/// Writes each of `frames` out as a PNG under `into`, creating the directory.
///
/// Asked-for frames are sorted and de-duplicated, so a caller that names the
/// same instant twice — a boundary that is also a time it asked about — decodes
/// it once and gets one file. The names are zero-padded so a directory listing
/// is in playback order.
pub fn stills(
    tools: &Tools,
    file: &Path,
    frames: &[u64],
    resolution: Resolution,
    into: &Path,
) -> Result<Vec<Still>, FrameError> {
    let mut wanted = frames.to_vec();
    wanted.sort_unstable();
    wanted.dedup();

    let mut written = Vec::with_capacity(wanted.len());
    for frame in wanted {
        let path = into.join(format!("frame-{frame:05}.png"));
        write_png(tools, &path, &extract(tools, file, frame, resolution)?)?;
        written.push(Still { frame, path });
    }
    Ok(written)
}
