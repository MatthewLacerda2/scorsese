//! Getting the samples of a finished file out of ffmpeg.
//!
//! Shared by the two things in this crate that *read* a file rather than make
//! one — the level report and the waveform. They ran the same decode twice, and
//! that is how they came to be wrong in the same way at the same time: both
//! asked ffmpeg for `-ac 1` into a float format, and libswresample normalises
//! its rematrix for integer output only, so the command *summed* the channels
//! instead of averaging them. Every number both tools printed was up to 6 dB
//! hot on centred content and they declared clipping on files that do not clip
//! (#452). One decode, in one place, is what stops the next correction having
//! to be made twice.
//!
//! **A file is measured as it is**, at its own channel count, rather than
//! folded down to one. That is not merely the cheaper fix, it is the honest
//! one: the peak is then the file's own peak, so the clipping verdict is about
//! the delivered file, and the mean is the number `volumedetect` and a bake's
//! own report already give — the two paths agree by construction rather than by
//! coincidence. Any downmix is a different signal. A *correct* average still
//! reads a hard-panned file 6 dB under itself, which gets the same verdict
//! wrong in the other direction, and it throws the channels away before the
//! meter, whose true-peak reconstruction is per-channel and needs the real
//! interleave to mean anything.
//!
//! What stays true is that a **report is one table**: nothing here measures the
//! channels separately or prints a row per channel. Loudness and spectral
//! balance are questions about the file, and the finding — this stretch is
//! quiet, this one is muddy — is the same finding in both channels.

use std::io::Read;
use std::path::Path;
use std::process::Stdio;

use scorsese_core::probe::ProbeMedia;

use crate::error::{RenderError, Stage};
use crate::pipe::SAMPLE_FORMAT;
use crate::probe::Ffprobe;
use crate::tools::Tools;

/// The rate a file is analysed at, whatever it was recorded at.
///
/// Fixed rather than taken from the source, so two files compared with each
/// other are measured on one clock. A resample changes no answer this reports —
/// mean, peak and three band shares are all properties of the waveform rather
/// than of the grid it is sampled on.
pub(crate) const ANALYSIS_RATE: u32 = 48_000;

/// How many bytes to pull from ffmpeg at a time.
const CHUNK: usize = 1 << 16;

/// How many interleaved channels `file` carries.
///
/// Asked of ffprobe rather than assumed, because a meter has to be told and
/// getting it wrong is not a rounding error: interpolating between a left
/// sample and the right sample beside it is interpolating between two different
/// signals, and would invent excursions at every frame boundary.
///
/// One for a file ffprobe cannot read, and one for a file with no sound in it
/// at all. Neither case has samples to mis-interleave, and the decode that
/// follows is what reports a file nothing can open — a report that failed here
/// instead would blame the probe for a broken file.
pub(crate) fn channels(tools: &Tools, file: &Path) -> usize {
    Ffprobe::new(tools.clone())
        .probe(file)
        .ok()
        .and_then(|media| media.audio_channels)
        .map_or(1, |channels| usize::from(channels).max(1))
}

/// Decodes every sample of `file` at [`ANALYSIS_RATE`], handing them to `take`
/// a run at a time.
///
/// A run at a time rather than a buffer, because one caller wants the samples
/// and the other only wants statistics over them: an hour of stereo held whole
/// to learn three numbers is six hundred megabytes for nothing.
///
/// `channels` is asked for rather than worked out here so that what the meter
/// was told and what ffmpeg was told are the same number by construction.
pub(crate) fn decode(
    tools: &Tools,
    file: &Path,
    channels: usize,
    mut take: impl FnMut(&[f32]),
) -> Result<(), RenderError> {
    let subject = file.display().to_string();
    let mut command = tools.ffmpeg();
    command
        .args(["-nostdin", "-v", "error"])
        .arg("-i")
        .arg(file)
        .args(["-vn", "-ar", &ANALYSIS_RATE.to_string()])
        .args(["-ac", &channels.to_string()])
        .args(["-f", SAMPLE_FORMAT, "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().map_err(|source| RenderError::Spawn {
        stage: Stage::Measure,
        source,
    })?;
    let mut stdout = child
        .stdout
        .take()
        .expect("stdout was piped when the process was spawned");
    read_all(&mut stdout, channels, &mut take)?;
    // Waited on only after the pipe has run dry: ffmpeg blocks writing into a
    // full pipe, so a process we stopped reading from would never exit.
    crate::pipe::finish(child, Stage::Measure, &subject)
}

/// Reads every sample ffmpeg produces into `take`, a whole number of
/// sample-frames at a time.
///
/// A pipe is free to hand back any number of bytes, so what is left over at the
/// end of a read is carried into the next one. Dropping it would shift every
/// later sample, which is not a rounding error but a different signal.
///
/// **Frames and not words**, which is the part that only matters once there is
/// more than one channel: a run ending half way through a frame would hand a
/// left sample over without the right one beside it, and every later run would
/// have its channels the wrong way round. A meter reconstructing one channel
/// then interpolates across two different signals and invents an excursion at
/// the seam — a true peak that is a fact about the buffering rather than about
/// the file.
fn read_all(
    source: &mut impl Read,
    channels: usize,
    take: &mut impl FnMut(&[f32]),
) -> Result<(), RenderError> {
    let frame = size_of::<f32>() * channels;
    let mut bytes = vec![0_u8; CHUNK.max(frame)];
    let mut carried = 0;
    let mut samples = Vec::with_capacity(bytes.len() / size_of::<f32>());
    loop {
        let read = source
            .read(&mut bytes[carried..])
            .map_err(|source| RenderError::Pipe {
                stage: Stage::Measure,
                source,
            })?;
        if read == 0 {
            return Ok(());
        }
        let filled = carried + read;
        let whole = filled / frame * frame;
        if whole > 0 {
            samples.clear();
            samples.extend(
                bytes[..whole]
                    .chunks_exact(size_of::<f32>())
                    .map(|word| f32::from_le_bytes(word.try_into().expect("four bytes"))),
            );
            take(&samples);
        }
        bytes.copy_within(whole..filled, 0);
        carried = filled - whole;
    }
}
