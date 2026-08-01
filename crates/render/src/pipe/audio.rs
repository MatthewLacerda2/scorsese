//! Getting raw samples out of a source file.
//!
//! The audio counterpart of [`super::decode`], and deliberately the same shape:
//! ffmpeg decodes and resamples, we do the rest. Everything arriving here is
//! already at the render's sample rate and channel count, so the mixer only
//! ever adds numbers that line up.

use std::io::{ErrorKind, Read};
use std::path::PathBuf;
use std::process::{Child, ChildStdout, Stdio};

use scorsese_core::Speed;

use crate::audio::CHANNELS;
use crate::error::{RenderError, Stage};
use crate::settings::RenderSettings;
use crate::tools::Tools;

/// The raw sample format we ask ffmpeg for and hand back to it.
///
/// Float, because the mixer sums several sources and integers would have to
/// deal with the overshoot at every addition rather than once at the end.
pub(crate) const SAMPLE_FORMAT: &str = "f32le";

/// How much longer than asked for to let ffmpeg decode.
///
/// `-t` bounds the work so a two-second segment of a five-minute music bed does
/// not decode five minutes; the margin covers the sample or two that rounding a
/// duration into seconds can cost, since coming up short would be silence in the
/// mix rather than a rounding error.
const MARGIN_SECONDS: f64 = 0.5;

/// What to decode, and how much of it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AudioSource {
    /// The media to read, already resolved against the project root — nothing
    /// below here knows what a project directory is.
    pub(crate) file: PathBuf,
    /// Where to start in the source, in wall-clock seconds.
    pub(crate) seek_seconds: f64,
    /// How fast to run the source. The clip's own speed, applied to its sound
    /// exactly as it is to its picture, and — per the format — **taking the
    /// pitch with it**.
    pub(crate) speed: Speed,
    /// How many sample-frames to ask for, at the render's rate.
    pub(crate) frames: u64,
}

/// An ffmpeg process decoding one source into raw samples on its stdout.
pub(crate) struct AudioDecoder {
    child: Child,
    stdout: ChildStdout,
    subject: String,
}

impl AudioDecoder {
    /// Starts decoding, resampled to the render's rate and downmixed or
    /// upmixed to its channel count.
    pub(crate) fn start(
        tools: &Tools,
        source: &AudioSource,
        settings: &RenderSettings,
    ) -> Result<Self, RenderError> {
        let rate = settings.sample_rate.hz();
        let seconds = source.frames as f64 / f64::from(rate) + MARGIN_SECONDS;

        let mut command = tools.ffmpeg();
        command.args(["-nostdin", "-v", "error"]);
        if source.seek_seconds > 0.0 {
            command
                .arg("-ss")
                .arg(format!("{:.6}", source.seek_seconds));
        }
        command.arg("-i").arg(&source.file);
        if let Some(filter) = speed_filter(source.speed, rate) {
            command.arg("-af").arg(filter);
        }
        command
            .args(["-t", &format!("{seconds:.6}")])
            .args(["-vn", "-ar", &rate.to_string()])
            .args(["-ac", &CHANNELS.to_string()])
            .args(["-f", SAMPLE_FORMAT, "-"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|source| RenderError::Spawn {
            stage: Stage::Mix,
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

    /// Reads up to `frames` sample-frames into `out`, replacing what is there.
    ///
    /// Returns how many arrived. Fewer than asked for means the source ran out,
    /// which is a fact about the media rather than a failure — the caller
    /// reports it and the gap stays silent.
    pub(crate) fn read(&mut self, out: &mut Vec<f32>, frames: usize) -> Result<usize, RenderError> {
        let mut bytes = vec![0_u8; frames * CHANNELS * size_of::<f32>()];
        let filled = fill(&mut self.stdout, &mut bytes).map_err(|source| RenderError::Pipe {
            stage: Stage::Mix,
            source,
        })?;

        out.clear();
        out.extend(
            bytes[..filled]
                .chunks_exact(size_of::<f32>())
                .map(|word| f32::from_le_bytes(word.try_into().expect("four bytes"))),
        );
        // A partial sample-frame cannot be mixed: it would put one channel of an
        // instant into the buffer without the other, shifting every later
        // sample by a channel.
        let complete = out.len() / CHANNELS * CHANNELS;
        out.truncate(complete);
        Ok(complete / CHANNELS)
    }

    /// Waits for ffmpeg and reports what it said if it failed.
    pub(crate) fn finish(mut self) -> Result<(), RenderError> {
        // Drain first: ffmpeg blocks writing into a full pipe, so waiting on a
        // process we stopped reading from would deadlock rather than exit.
        std::io::copy(&mut self.stdout, &mut std::io::sink()).map_err(|source| {
            RenderError::Pipe {
                stage: Stage::Mix,
                source,
            }
        })?;
        super::finish(self.child, Stage::Mix, &self.subject)
    }
}

/// The filter that plays a clip's sound at its own rate — `None` when that rate
/// is the source's own and there is nothing to do.
///
/// **The pitch goes with the speed**, which is the format's decision and not an
/// accident of the implementation. This is the record-player answer: the samples
/// are left exactly as they are and the clock underneath them is moved, so a
/// clip at 2× is an octave up. Holding the pitch would mean a time-stretch — a
/// real DSP problem, a new dependency, and its own artefacts — and it is a
/// separate piece of work rather than a flag here.
///
/// Three stages, and the first is the one that looks redundant. `asetrate` takes
/// a sample rate as a number, and the rate the *source* happens to be at is not
/// something this process knows; pinning the filter's input to the render's rate
/// first is what makes the multiplication expressible at all. `asetrate` then
/// relabels those samples as arriving `speed` times faster, and the `-ar` the
/// decoder already asks for resamples them back — so what reaches the mixer is
/// at one rate, as it always was.
///
/// The played rate is rounded to a whole number of hertz, which is the one
/// inexactness here: at 48 kHz that is at worst a hundred-thousandth of the
/// requested speed, or a frame of drift every eleven hours.
fn speed_filter(speed: Speed, rate: u32) -> Option<String> {
    if speed.is_normal() || !speed.is_usable() {
        return None;
    }
    let played = (f64::from(rate) * speed.get())
        .round()
        .clamp(1.0, f64::from(u32::MAX)) as u32;
    Some(format!("aresample={rate},asetrate={played}"))
}

/// Reads until `buffer` is full or the source ends, returning how many bytes
/// arrived. `Read::read` is free to return less than asked for at any time, so
/// a single call is not an answer about whether the stream ended.
fn fill(reader: &mut impl Read, buffer: &mut [u8]) -> std::io::Result<usize> {
    let mut filled = 0;
    while filled < buffer.len() {
        match reader.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(read) => filled += read,
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(filled)
}
