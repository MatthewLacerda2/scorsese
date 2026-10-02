//! The ffmpeg processes a render talks to.
//!
//! This is the whole of ffmpeg's job in scorsese: hand us raw frames and raw
//! samples from a source, and take them back to encode. Everything between —
//! what is on screen, where, how opaque, and how loud — happens in our process.
//! That is Path B, and these files are its edges.

mod audio;
mod decode;
mod encode;

pub(crate) use audio::{AudioDecoder, AudioSource, SAMPLE_FORMAT};
pub(crate) use decode::{Decoder, Fitting, Source, reads_through_image2};
pub(crate) use encode::{Encoder, encode_mix};

use std::process::Child;
use std::time::{Duration, Instant};

use crate::error::{RenderError, Stage};

/// An ffmpeg child that is reaped however the render that spawned it ends.
///
/// Dropping a [`Child`] neither kills nor waits for it, so a render that
/// returned early — a failure, a cancel (#647) — used to leave its decoders and
/// its encoder running, or as zombies under a server that lives for hours. A
/// process that is not [`Process::finish`]ed or [`Process::stop`]ped on purpose
/// is killed and waited for when this drops.
pub(crate) struct Process(Option<Child>);

impl Process {
    /// Takes charge of a freshly spawned child.
    pub(crate) fn new(child: Child) -> Self {
        Self(Some(child))
    }

    /// Waits for it to end on its own, as [`finish`] does.
    pub(crate) fn finish(mut self, stage: Stage, subject: &str) -> Result<(), RenderError> {
        let child = self.0.take().expect("a process is only finished once");
        finish(child, stage, subject)
    }

    /// Gives it `grace` to exit by itself — its stdin already closed, which is
    /// how ffmpeg is asked to stop — and kills it when it has not.
    ///
    /// Patient first because a polite exit is ffmpeg's own; the kill is for
    /// one that is wedged, as the encoder in #647 was, ignoring everything
    /// short of `SIGKILL`.
    pub(crate) fn stop(mut self, grace: Duration) {
        let Some(mut child) = self.0.take() else {
            return;
        };
        let deadline = Instant::now() + grace;
        while Instant::now() < deadline {
            match child.try_wait() {
                Ok(Some(_)) | Err(_) => return,
                Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            }
        }
        let _ = child.kill();
        let _ = child.wait();
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

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
