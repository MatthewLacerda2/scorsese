//! Putting raw frames into a file.

use std::io::Write;
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};

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
            mix_input(&mut command, settings, mix);
        }
        let format = settings.format;
        let video = format
            .video()
            .expect("picture is only ever encoded for a format that has one");
        command.args(["-c:v", video.encoder()]);
        command.args(["-pix_fmt", ENCODED_PIXELS]);
        // A size budget is the exception, not the rule — most renders want
        // "look right", and a fixed bitrate spends the same bits on a still
        // frame as on a hard cut. Which knob asks for that is the codec's own
        // business: `-crf` is x264's and means nothing to the other two.
        match settings.bitrate {
            Some(bitrate) => command.args(["-b:v", &bitrate.ffmpeg_value()]),
            None => command.args(video.quality()),
        };
        match mix {
            // The raw float samples we mixed in would be enormous and
            // unplayable on half the devices this has to reach, so the mix is
            // always encoded as whatever the container is written with.
            Some(_) => audio_codec(&mut command, settings),
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

/// Encodes a finished mix on its own, into `out`, exactly as a render would
/// encode it beside picture — same codec, same bitrate, same container.
///
/// A rehearsal for the real encode, which is what makes it worth running:
/// how far a lossy codec overshoots depends on the material, so the only way
/// to know what the delivered soundtrack will peak at is to encode this one
/// and look. Audio alone is seconds of work against the minutes the picture
/// costs, and the audio encoder does not know or care that a video stream is
/// muxed beside it.
///
/// It is also the whole encode of a delivery with no picture in it, which is
/// why the rehearsal and the real thing cannot drift: they are one call.
pub(crate) fn encode_mix(
    tools: &Tools,
    settings: &RenderSettings,
    mix: &Path,
    out: &Path,
) -> Result<(), RenderError> {
    let mut command = tools.ffmpeg();
    command.args(["-nostdin", "-v", "error", "-y"]);
    mix_input(&mut command, settings, mix);
    audio_codec(&mut command, settings);
    command
        .args(["-f", settings.format.container().muxer()])
        .arg(out)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let child = command.spawn().map_err(|source| RenderError::Spawn {
        stage: Stage::Encode,
        source,
    })?;
    super::finish(child, Stage::Encode, &out.display().to_string())
}

/// Hands a finished mix to ffmpeg as an input: raw float samples, at the
/// render's rate and our channel count, which it has no header to learn from.
fn mix_input(command: &mut Command, settings: &RenderSettings, mix: &Path) {
    command
        .args(["-f", SAMPLE_FORMAT])
        .args(["-ar", &settings.sample_rate.hz().to_string()])
        .args(["-ac", &CHANNELS.to_string()])
        .arg("-i")
        .arg(mix);
}

/// Asks for the audio codec the container is written with, and the bitrate
/// when one was chosen and the codec has any use for it.
///
/// One place for both encodes, so the rehearsal in [`encode_mix`] cannot
/// quietly encode differently from the delivery it is standing in for.
fn audio_codec(command: &mut Command, settings: &RenderSettings) {
    let codec = settings.format.audio();
    command.args(["-c:a", codec.encoder()]);
    // Uncompressed audio has exactly one rate, so `-b:a` against it is a
    // setting that would silently do nothing.
    if let (Some(bitrate), true) = (settings.audio_bitrate, codec.takes_bitrate()) {
        command.args(["-b:a", &bitrate.ffmpeg_value()]);
    }
}
