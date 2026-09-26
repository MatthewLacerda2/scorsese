//! A small picture of a media file, for a list of them to show.
//!
//! The web app's library (#535) shows each file as a thumbnail beside its name,
//! and a thumbnail is ffmpeg work — a decoded frame, scaled — so it is here,
//! behind the command builder like every other invocation, rather than in the
//! server that asks for it.
//!
//! One command per kind of file, and the output's format is whatever the
//! caller's file name says, because ffmpeg picks the encoder from it: a video's
//! frame reads well as a JPEG, while a picture may carry transparency and a
//! waveform is flat colour, both of which a PNG keeps and compresses better.
//! Which to use is the caller's choice ([`Thumbnail::extension`] is the
//! suggestion), so this never guesses at a name.

use std::path::Path;
use std::process::Stdio;

use crate::tools::Tools;

use super::{FrameError, stderr_of};

/// The longest side of a thumbnail, in pixels. Nothing is enlarged to reach it.
pub const THUMBNAIL_SIZE: u32 = 320;

/// What a thumbnail is made of.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Thumbnail {
    /// One frame of a video, this many seconds in.
    Frame {
        /// Where to take it. Past the end of the file there is no frame, and
        /// the command fails rather than inventing one.
        at_seconds: f64,
    },
    /// A still picture, scaled down.
    Picture,
    /// A sound, drawn as its waveform.
    Waveform,
}

impl Thumbnail {
    /// The file extension this kind of thumbnail is best written with.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Frame { .. } => "jpg",
            Self::Picture | Self::Waveform => "png",
        }
    }
}

/// Writes a thumbnail of `source` to `out`, replacing whatever is there, in the
/// format `out`'s extension names.
pub fn thumbnail(
    tools: &Tools,
    source: &Path,
    what: Thumbnail,
    out: &Path,
) -> Result<(), FrameError> {
    let io = |source| FrameError::Io {
        file: out.to_path_buf(),
        source,
    };
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(io)?;
    }
    let side = THUMBNAIL_SIZE;
    // Fits inside a square of `side`, keeping the shape, never enlarging, and
    // even on both sides — which every encoder accepts.
    let fit = format!(
        "scale='min({side},iw)':'min({side},ih)':force_original_aspect_ratio=decrease\
         :force_divisible_by=2"
    );
    let mut command = tools.ffmpeg();
    command.args(["-nostdin", "-v", "error", "-y"]);
    match what {
        Thumbnail::Frame { at_seconds } => {
            command
                .args(["-ss", &format!("{:.3}", at_seconds.max(0.0))])
                .arg("-i")
                .arg(source)
                .args(["-vf", &fit, "-q:v", "4"]);
        }
        Thumbnail::Picture => {
            command.arg("-i").arg(source).args(["-vf", &fit]);
        }
        Thumbnail::Waveform => {
            let height = side * 3 / 8;
            command.arg("-i").arg(source).args([
                "-filter_complex",
                &format!("showwavespic=s={side}x{height}:split_channels=0:colors=0x8a8f98"),
            ]);
        }
    }
    let output = command
        .args(["-frames:v", "1", "-update", "1"])
        .arg(out)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(io)?;
    // ffmpeg can exit cleanly having written nothing — a seek past the end
    // finds no frame — so success is the file, not the status alone.
    if output.status.success() && out.metadata().is_ok_and(|meta| meta.len() > 0) {
        return Ok(());
    }
    let message = if output.status.success() {
        "ffmpeg found no frame to draw".to_owned()
    } else {
        stderr_of(&output.stderr)
    };
    Err(FrameError::Ffmpeg {
        file: source.to_path_buf(),
        message,
    })
}
