//! Putting raw frames into a file.

use std::io::Write;
use std::path::Path;
use std::process::{Child, ChildStdin, Stdio};

use super::audio::SAMPLE_FORMAT;
use crate::audio::CHANNELS;
use crate::error::{RenderError, Stage};
use crate::settings::RenderSettings;
use crate::tools::Tools;
use scorsese_compositor::{Frame, PIXEL_FORMAT};

/// Constant-quality target used when no bitrate is asked for. A size budget is
/// the exception, not the rule — most renders want "look right", and a fixed
/// bitrate spends the same bits on a still frame as on a hard cut.
const DEFAULT_CRF: &str = "18";

/// An ffmpeg process taking raw frames on its stdin and writing an encoded
/// file.
pub struct Encoder {
    child: Child,
    stdin: ChildStdin,
    subject: String,
}

impl Encoder {
    /// Starts encoding to `out`, optionally muxing in a finished mix.
    ///
    /// H.264 in whatever container `out`'s extension implies, 4:2:0 — the
    /// combination that plays everywhere. Codec choice is not a render setting
    /// yet because nothing has needed a second one; when something does, it
    /// belongs next to the settings this takes, not hardcoded further up.
    ///
    /// The mix arrives as a **file** while picture arrives on stdin, for the
    /// blunt reason that a process has one stdin. It is already complete by the
    /// time this starts, so ffmpeg reads it at whatever pace it encodes at and
    /// nothing has to be kept in step.
    pub fn start(
        tools: &Tools,
        settings: &RenderSettings,
        mix: Option<&Path>,
        out: &Path,
    ) -> Result<Self, RenderError> {
        let mut command = tools.ffmpeg();
        command
            .args(["-nostdin", "-v", "error", "-y"])
            .args(["-f", "rawvideo", "-pix_fmt", PIXEL_FORMAT])
            .args(["-s", &settings.resolution.to_string()])
            .args([
                "-r",
                &format!("{}/{}", settings.fps.num(), settings.fps.den()),
            ])
            .args(["-i", "-"]);
        if let Some(mix) = mix {
            command
                .args(["-f", SAMPLE_FORMAT])
                .args(["-ar", &settings.sample_rate.hz().to_string()])
                .args(["-ac", &CHANNELS.to_string()])
                .arg("-i")
                .arg(mix);
        }
        command.args(["-c:v", "libx264", "-pix_fmt", "yuv420p"]);
        match settings.bitrate {
            Some(bitrate) => command.args(["-b:v", &bitrate.ffmpeg_value()]),
            None => command.args(["-crf", DEFAULT_CRF]),
        };
        match mix {
            // AAC because it is what an .mp4 is expected to carry and what every
            // player decodes; the raw float samples we mixed in would be
            // enormous and unplayable on half the devices this has to reach.
            Some(_) => {
                command.args(["-c:a", "aac"]);
                if let Some(bitrate) = settings.audio_bitrate {
                    command.args(["-b:a", &bitrate.ffmpeg_value()]);
                }
            }
            None => {
                command.arg("-an");
            }
        }
        command
            .arg(out)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|source| RenderError::Spawn {
            stage: Stage::Encode,
            source,
        })?;
        let stdin = child
            .stdin
            .take()
            .expect("stdin was piped when the process was spawned");
        Ok(Self {
            child,
            stdin,
            subject: out.display().to_string(),
        })
    }

    /// Hands one finished frame to the encoder.
    pub fn write(&mut self, frame: &Frame) -> Result<(), RenderError> {
        self.stdin
            .write_all(frame.bytes())
            .map_err(|source| RenderError::Pipe {
                stage: Stage::Encode,
                source,
            })
    }

    /// Closes the pipe and waits for the file to be finalised.
    pub fn finish(self) -> Result<(), RenderError> {
        let Self {
            child,
            stdin,
            subject,
        } = self;
        // Closing stdin is what tells ffmpeg the stream ended; without it,
        // waiting for the process would wait forever.
        drop(stdin);
        super::finish(child, Stage::Encode, &subject)
    }
}
