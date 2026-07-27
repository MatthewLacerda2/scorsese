//! Getting raw frames out of a source file.

use std::io::{ErrorKind, Read};
use std::path::PathBuf;
use std::process::{Child, ChildStdout, Stdio};

use crate::error::{RenderError, Stage};
use crate::settings::RenderSettings;
use crate::tools::Tools;
use scorsese_compositor::{Frame, PIXEL_FORMAT, Resolution};

/// How a source meets the render's raster, resolved to pixels.
///
/// [`scorsese_core::Fit`] says what the author asked for; this is what the
/// decoder does about it, which for one of the three means already knowing the
/// source's own size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fitting {
    /// Scale to fit inside the raster; pad the rest transparent.
    Fit,
    /// Scale to cover the raster; crop the overflow off the edges.
    Fill,
    /// Leave the source alone. It arrives at this — its own — size.
    Native(Resolution),
}

impl Fitting {
    /// The size frames come out at, and so the size of the buffer that reads
    /// them. Fitting and filling produce the render's raster by construction;
    /// native produces whatever the source happens to be.
    pub fn raster(self, settings: &RenderSettings) -> Resolution {
        match self {
            Self::Fit | Self::Fill => settings.resolution,
            Self::Native(source) => source,
        }
    }
}

/// What to decode, and how much of it.
#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    /// The media to read, already resolved against the project root — nothing
    /// below here knows what a project directory is.
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
    /// How this source meets the raster.
    pub fitting: Fitting,
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
    raster: Resolution,
}

impl Decoder {
    /// Starts decoding. Frames arrive re-timed to the render's framerate and
    /// fitted the way the clip asked to be: fitting a source into the output
    /// raster is a decode concern, and doing it here means every frame reaching
    /// our process is already the size the compositor will place.
    ///
    /// Sources of a different shape are never stretched. `fit` letterboxes them
    /// — a vertical phone clip in a 16:9 render gets transparent at the sides —
    /// `fill` crops instead, and `native` leaves the source at its own size for
    /// the compositor to rest on the canvas.
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
            .arg(video_filter(settings, source.fitting))
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
            raster: source.fitting.raster(settings),
        })
    }

    /// The size the frames it produces are. Asked of the decoder rather than
    /// worked out again by the caller, so a buffer can only ever be the size
    /// the pipe is about to fill — a mismatch would not fail, it would slide
    /// every later frame along by the difference.
    pub const fn raster(&self) -> Resolution {
        self.raster
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

/// Re-time first, then fit — in that order, because dropping frames before
/// scaling means not scaling the frames that get dropped.
///
/// `fit` letterboxes and the bars are **transparent**, not black; `format=rgba`
/// before the pad is what keeps them so. On the bottom layer the difference is
/// invisible — the canvas underneath is black anyway — but on an upper track it
/// is the whole point: a narrow clip over a wide one shows the wide one at the
/// sides rather than blacking it out.
///
/// `fill` is the same scale with the rounding turned the other way, so the
/// source covers the raster instead of sitting inside it, and a centred crop
/// takes the overflow off. Nothing is padded because nothing is left over.
///
/// `native` scales nothing and pads nothing: the frames come out at the source's
/// own size, and where they sit on the canvas is the compositor's business.
fn video_filter(settings: &RenderSettings, fitting: Fitting) -> String {
    let rate = format!("fps={}/{}", settings.fps.num(), settings.fps.den());
    let width = settings.resolution.width();
    let height = settings.resolution.height();
    match fitting {
        Fitting::Fit => format!(
            "{rate},\
             scale={width}:{height}:force_original_aspect_ratio=decrease,\
             format=rgba,\
             pad={width}:{height}:(ow-iw)/2:(oh-ih)/2:color=black@0.0"
        ),
        Fitting::Fill => format!(
            "{rate},\
             scale={width}:{height}:force_original_aspect_ratio=increase,\
             format=rgba,\
             crop={width}:{height}"
        ),
        Fitting::Native(_) => format!("{rate},format=rgba"),
    }
}
