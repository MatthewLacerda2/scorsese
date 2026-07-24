//! Getting raw frames out of a source file.

use std::io::{ErrorKind, Read};
use std::path::PathBuf;
use std::process::{Child, ChildStdout, Stdio};

use crate::error::{RenderError, Stage};
use crate::frame::{Frame, PIXEL_FORMAT};
use crate::settings::RenderSettings;
use crate::tools::Tools;

/// What to decode, and how much of it.
#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    pub file: PathBuf,
    /// A still image, which has no timeline of its own and is held for as long
    /// as the clip lasts.
    pub still: bool,
    /// Where to start in the source, in wall-clock seconds. Seconds because
    /// that is the unit ffmpeg seeks in; the conversion from the timeline grid
    /// happened before we got here.
    pub seek_seconds: f64,
    /// How many frames to ask for, on the output grid.
    pub frames: u64,
}

/// An ffmpeg process decoding one source into raw frames on its stdout.
///
/// One process per segment rather than one for the whole timeline: a segment
/// has one source, one seek point, and one frame count, and a process per
/// segment keeps that mapping obvious. Sequencing several sources into one
/// ffmpeg invocation would be handing our job — deciding what is on screen
/// when — back to ffmpeg.
pub struct Decoder {
    child: Child,
    stdout: ChildStdout,
    subject: String,
}

impl Decoder {
    /// Starts decoding. Frames arrive already scaled to the render's
    /// resolution and re-timed to its framerate: fitting a source into the
    /// output raster is a decode concern, and doing it in the decoder means
    /// every frame reaching our process is already the size we composite at.
    ///
    /// Sources of a different shape are letterboxed, never stretched — a
    /// vertical phone clip in a 16:9 render gets black at the sides.
    pub fn start(
        tools: &Tools,
        source: &Source,
        settings: &RenderSettings,
    ) -> Result<Self, RenderError> {
        let rate = format!("{}/{}", settings.fps.num(), settings.fps.den());
        let mut command = tools.ffmpeg();
        command.args(["-nostdin", "-v", "error"]);
        if source.still {
            command.args(["-loop", "1", "-framerate", &rate]);
        } else if source.seek_seconds > 0.0 {
            command
                .arg("-ss")
                .arg(format!("{:.6}", source.seek_seconds));
        }
        command
            .arg("-i")
            .arg(&source.file)
            .args(["-frames:v", &source.frames.to_string()])
            .arg("-vf")
            .arg(video_filter(settings))
            .args(["-an", "-pix_fmt", PIXEL_FORMAT, "-f", "rawvideo", "-"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|source| RenderError::Spawn {
            stage: Stage::Decode,
            source,
        })?;
        let stdout = child
            .stdout
            .take()
            .expect("stdout was piped when the process was spawned");
        Ok(Self {
            child,
            stdout,
            subject: source.file.display().to_string(),
        })
    }

    /// Reads the next frame into `frame`. `false` means the source ran out —
    /// which is a fact about the media, not a failure: a clip longer than its
    /// source is a project mistake, and the caller reports it.
    pub fn read_into(&mut self, frame: &mut Frame) -> Result<bool, RenderError> {
        match self.stdout.read_exact(frame.bytes_mut()) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == ErrorKind::UnexpectedEof => Ok(false),
            Err(source) => Err(RenderError::Pipe {
                stage: Stage::Decode,
                source,
            }),
        }
    }

    /// Waits for ffmpeg and reports what it said if it failed.
    pub fn finish(mut self) -> Result<(), RenderError> {
        // Drain anything still in flight first. ffmpeg blocks writing into a
        // full pipe, so waiting on a process we stopped reading from would
        // deadlock rather than exit.
        std::io::copy(&mut self.stdout, &mut std::io::sink()).map_err(|source| {
            RenderError::Pipe {
                stage: Stage::Decode,
                source,
            }
        })?;
        super::finish(self.child, Stage::Decode, &self.subject)
    }
}

/// Re-time, fit, letterbox — in that order, because dropping frames before
/// scaling means not scaling the frames that get dropped.
fn video_filter(settings: &RenderSettings) -> String {
    let width = settings.resolution.width();
    let height = settings.resolution.height();
    format!(
        "fps={}/{},\
         scale={width}:{height}:force_original_aspect_ratio=decrease,\
         pad={width}:{height}:(ow-iw)/2:(oh-ih)/2:color=black",
        settings.fps.num(),
        settings.fps.den()
    )
}
