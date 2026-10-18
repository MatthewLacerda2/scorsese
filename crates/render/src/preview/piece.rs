//! A preview played in pieces (#1448): the remux that makes a finished render
//! of a few seconds one a browser can append to the pieces before it.
//!
//! A long cut is previewed the way a video site plays one: a few seconds at a
//! time from the playhead forward, each playable the moment it lands. A piece
//! is an ordinary render over a range — picture and sound together, so
//! neither waits on the whole timeline — remuxed into **fragmented MP4**,
//! which is what Media Source Extensions takes.
//!
//! Two things make the joins seamless, both measured in Chromium:
//!
//! - **The picture's first frame is at zero.** Its composition offsets are
//!   written signed, rather than shifting the whole piece later by the
//!   encoder's reordering delay — a gap of a frame or two at every join,
//!   which a browser stalls on.
//! - **The sound is FLAC.** A piece is rendered with plain PCM sound and
//!   only then made FLAC: lossless, with no priming, so each piece decodes to
//!   exactly its own samples and the next begins where it ended. An AAC piece
//!   starts with the encoder's priming, a click (or a stall) at every join.
//!
//! The picture is copied, never re-encoded. Every piece of a cut with any
//! sound carries a sound track, silent where its stretch has none, because a
//! browser's buffer takes the same tracks in every piece it is given.

use std::path::Path;
use std::process::Stdio;

use crate::cancel::Cancel;
use crate::error::{RenderError, Stage};
use crate::pipe::{Partial, finish_unless};
use crate::settings::SampleRate;
use crate::tools::Tools;

/// What sound a piece carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sound {
    /// None: the cut has nothing to hear anywhere.
    Absent,
    /// The render's own mix, made FLAC.
    Mixed,
    /// Silence this many samples long at this rate: the cut has sound, but
    /// not in this stretch.
    Silent(SampleRate, u64),
}

/// The movflags a piece is written with: fragments at each keyframe behind
/// an empty header, each fragment's times its own, and composition offsets
/// that may go negative so the first frame shown is at zero.
const FRAGMENTED: &str = "frag_keyframe+empty_moov+default_base_moof+negative_cts_offsets";

/// Remuxes the finished render at `from` into a fragmented MP4 piece at
/// `out` carrying `sound`, written beside it and renamed last like every
/// delivery here. Tripping `cancel` stops it and leaves nothing at `out`.
pub fn fragment(
    tools: &Tools,
    from: &Path,
    sound: Sound,
    out: &Path,
    cancel: &Cancel,
) -> Result<(), RenderError> {
    let partial = Partial::beside(out);
    let mut command = tools.ffmpeg();
    command
        .args(["-nostdin", "-v", "error", "-y"])
        // An avi render carries no presentation times; made up again from its
        // frame order, or the reordered frames reach the browser untimed.
        .args(["-fflags", "+genpts", "-i"])
        .arg(from);
    if let Sound::Silent(rate, samples) = sound {
        let silence = format!(
            "anullsrc=r={}:cl=stereo,atrim=end_sample={samples}",
            rate.hz()
        );
        command.args(["-f", "lavfi", "-i", &silence]);
    }
    command.args(["-map", "0:v:0", "-c:v", "copy"]);
    match sound {
        Sound::Absent => {}
        Sound::Mixed => {
            command.args(["-map", "0:a:0", "-c:a", "flac"]);
        }
        Sound::Silent(..) => {
            command.args(["-map", "1:a:0", "-c:a", "flac"]);
        }
    }
    command
        .args(["-movflags", FRAGMENTED, "-f", "mp4"])
        .arg(partial.path())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let child = command.spawn().map_err(|source| RenderError::Spawn {
        stage: Stage::Encode,
        source,
    })?;
    finish_unless(child, cancel, Stage::Encode, &out.display().to_string())?;
    partial.deliver()
}
