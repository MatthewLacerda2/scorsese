//! The ffmpeg processes a render talks to.
//!
//! This is the whole of ffmpeg's job in scorsese: hand us raw frames and raw
//! samples from a source, and take them back to encode. Everything between —
//! what is on screen, where, how opaque, and how loud — happens in our process.
//! That is Path B, and these files are its edges.

mod audio;
mod decode;
mod encode;

pub use audio::{AudioDecoder, AudioSource, SAMPLE_FORMAT};
pub use decode::{Decoder, Fitting, Source};
pub use encode::Encoder;

use std::process::Child;

use crate::error::{RenderError, Stage};

/// Waits for an ffmpeg process and turns a non-zero exit into an error
/// carrying whatever it said on stderr.
pub(crate) fn finish(child: Child, stage: Stage, subject: &str) -> Result<(), RenderError> {
    let output = child
        .wait_with_output()
        .map_err(|source| RenderError::Pipe { stage, source })?;
    if output.status.success() {
        return Ok(());
    }
    let message = String::from_utf8_lossy(&output.stderr);
    let message = message.trim();
    Err(RenderError::Ffmpeg {
        stage,
        subject: subject.to_owned(),
        message: if message.is_empty() {
            format!("exited with {}", output.status)
        } else {
            message.to_owned()
        },
    })
}
