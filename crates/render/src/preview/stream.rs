//! A preview played in pieces (#1448): which stream a piece carries, and the
//! remux that makes a finished file one a browser can append.
//!
//! A long cut is previewed the way a video site plays one: a few seconds of
//! picture at a time from the playhead forward, each playable the moment it
//! lands, beside **one** sound file for the whole timeline. The picture is
//! cut into pieces because drawing it is what costs; the sound is not,
//! because mixing is cheap and an audio codec's priming at the head of every
//! piece is a click at every join. So a piece is one of two streams, each its
//! own file, and the player feeds them to the browser side by side.
//!
//! Both are **fragmented MP4** with the stream they carry and nothing else,
//! which is what Media Source Extensions takes. Each picture piece starts at
//! zero and the player places it on the timeline; its composition offsets are
//! written negative, so its first frame is at zero too rather than after the
//! encoder's reordering delay — without that, every join is a gap of a frame
//! or two that a browser stalls on.
//!
//! This is a remux and never a re-encode: the render's own encode is what is
//! kept, its packets copied into the new container.

use std::fmt;
use std::path::Path;
use std::process::Stdio;
use std::str::FromStr;

use crate::cancel::Cancel;
use crate::error::{RenderError, Stage};
use crate::pipe::{Partial, finish_unless};
use crate::tools::Tools;

/// Which stream of a preview played in pieces a file carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stream {
    /// A stretch of picture, with no sound.
    Picture,
    /// The whole timeline's sound, with no picture.
    Sound,
}

impl Stream {
    /// The name a request spells it with: `picture` or `sound`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Picture => "picture",
            Self::Sound => "sound",
        }
    }

    /// The ffmpeg stream specifier that picks it out of a render.
    const fn map(self) -> &'static str {
        match self {
            Self::Picture => "0:v:0",
            Self::Sound => "0:a:0",
        }
    }
}

impl fmt::Display for Stream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A stream name that is neither.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a preview stream: say picture or sound")]
pub struct StreamError(String);

impl FromStr for Stream {
    type Err = StreamError;

    fn from_str(text: &str) -> Result<Self, StreamError> {
        [Self::Picture, Self::Sound]
            .into_iter()
            .find(|stream| stream.name() == text)
            .ok_or_else(|| StreamError(text.to_owned()))
    }
}

/// The movflags a piece is written with: fragments at each keyframe behind
/// an empty header, each fragment's times its own, and composition offsets
/// that may go negative so the first frame shown is at zero.
const FRAGMENTED: &str = "frag_keyframe+empty_moov+default_base_moof+negative_cts_offsets";

/// Copies `stream` out of the finished render at `from` into a fragmented MP4
/// at `out`, written beside it and renamed last like every delivery here.
/// Tripping `cancel` stops it and leaves nothing at `out`.
pub fn fragment(
    tools: &Tools,
    from: &Path,
    stream: Stream,
    out: &Path,
    cancel: &Cancel,
) -> Result<(), RenderError> {
    let partial = Partial::beside(out);
    let mut command = tools.ffmpeg();
    command
        .args(["-nostdin", "-v", "error", "-y"])
        .arg("-i")
        .arg(from)
        .args(["-map", stream.map(), "-c", "copy"])
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stream_reads_back_from_its_own_name_and_nothing_else() {
        for stream in [Stream::Picture, Stream::Sound] {
            assert_eq!(stream.name().parse::<Stream>(), Ok(stream));
        }
        assert!("video".parse::<Stream>().is_err());
        assert!("".parse::<Stream>().is_err());
    }

    #[test]
    fn each_stream_picks_its_own_kind_out_of_a_render() {
        assert_eq!(Stream::Picture.map(), "0:v:0");
        assert_eq!(Stream::Sound.map(), "0:a:0");
    }
}
