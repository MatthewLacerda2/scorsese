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

/// The pixel format every encoder here is handed. 4:2:0 8-bit is what all
/// three of them take and what plays everywhere; a codec that wanted something
/// else would carry it on [`crate::format::VideoCodec`] rather than here.
const ENCODED_PIXELS: &str = "yuv420p";

/// An ffmpeg process taking raw frames on its stdin and writing an encoded
/// file.
pub(crate) struct Encoder {
    child: Child,
    stdin: ChildStdin,
    subject: String,
}

impl Encoder {
    /// Starts encoding to `out`, optionally muxing in a finished mix.
    ///
    /// The container and both codecs come from [`RenderSettings::format`], and
    /// the muxer is pinned with `-f` rather than left to ffmpeg to guess from
    /// `out`'s extension. Nothing here decides the shape of the file or checks
    /// it: an [`crate::format::OutputFormat`] only exists for a combination
    /// that was accepted, which is why by the time anything is spawned there is
    /// nothing left to refuse.
    ///
    /// The mix arrives as a **file** while picture arrives on stdin, for the
    /// blunt reason that a process has one stdin. It is already complete by the
    /// time this starts, so ffmpeg reads it at whatever pace it encodes at and
    /// nothing has to be kept in step.
    pub(crate) fn start(
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
        let format = settings.format;
        command.args(["-c:v", format.video().encoder()]);
        command.args(["-pix_fmt", ENCODED_PIXELS]);
        // A size budget is the exception, not the rule — most renders want
        // "look right", and a fixed bitrate spends the same bits on a still
        // frame as on a hard cut. Which knob asks for that is the codec's own
        // business: `-crf` is x264's and means nothing to the other two.
        match settings.bitrate {
            Some(bitrate) => command.args(["-b:v", &bitrate.ffmpeg_value()]),
            None => command.args(format.video().quality()),
        };
        match mix {
            // The raw float samples we mixed in would be enormous and
            // unplayable on half the devices this has to reach, so the mix is
            // always encoded as whatever the container is written with.
            Some(_) => {
                command.args(["-c:a", format.audio().encoder()]);
                match (settings.audio_bitrate, format.audio().takes_bitrate()) {
                    (Some(bitrate), true) => command.args(["-b:a", &bitrate.ffmpeg_value()]),
                    // Uncompressed audio has exactly one rate, so `-b:a`
                    // against it is a setting that would silently do nothing.
                    _ => &mut command,
                };
            }
            None => {
                command.arg("-an");
            }
        }
        command
            .args(["-f", format.container().muxer()])
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
    pub(crate) fn write(&mut self, frame: &Frame) -> Result<(), RenderError> {
        self.stdin
            .write_all(frame.bytes())
            .map_err(|source| RenderError::Pipe {
                stage: Stage::Encode,
                source,
            })
    }

    /// Closes the pipe and waits for the file to be finalised.
    pub(crate) fn finish(self) -> Result<(), RenderError> {
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
