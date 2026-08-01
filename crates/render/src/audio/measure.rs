//! Measuring a sound file that already exists.
//!
//! A render measures its mix as it builds it and a bake measures its samples
//! before it encodes them, so both get their numbers for free. This is the
//! third case: a file on disk, made earlier or by something else, that someone
//! wants the same numbers for.
//!
//! It exists because **the comparison is where a measurement becomes a
//! judgement**. Is −14 dBFS good? It depends entirely. Is it 4.6 dB under the
//! version it was meant to replace? That is a finding — and the version it was
//! meant to replace is a file, not a run in progress.
//!
//! ffmpeg decodes and we do the rest, exactly as everywhere else in this crate.
//! That is also what makes this work on a delivered `.mp4` as readily as on a
//! `.wav`: the analysis never learns what container it came out of.

use std::io::Read;
use std::path::Path;
use std::process::Stdio;

use scorsese_soundgen::level::{Cut, Profile, Profiler};

use crate::error::{RenderError, Stage};
use crate::pipe::SAMPLE_FORMAT;
use crate::tools::Tools;

/// The rate a file is analysed at, whatever it was recorded at.
///
/// Fixed rather than taken from the source, so two files compared with each
/// other are measured on one clock. A resample changes no answer this reports —
/// mean, peak and three band shares are all properties of the waveform rather
/// than of the grid it is sampled on.
pub const ANALYSIS_RATE: u32 = 48_000;

/// How many bytes to pull from ffmpeg at a time.
const CHUNK: usize = 1 << 16;

/// How `file` came out, over its whole length and section by section.
///
/// **Mono**, because loudness and spectral balance are the questions and a
/// downmix answers both for the whole file at once. Measuring the two channels
/// separately would produce two tables to read where the finding — this stretch
/// is quiet, this one is muddy — is the same in both.
pub fn measure(tools: &Tools, file: &Path) -> Result<Profile, RenderError> {
    profile(tools, file, Vec::new())
}

/// How `file` came out, cut where `sections` says rather than on the clock.
///
/// For a bake whose recipe is still in hand: the arrangement knows where its
/// patterns are, and a row that names one is worth several that name a time.
pub fn profile(tools: &Tools, file: &Path, sections: Vec<Cut>) -> Result<Profile, RenderError> {
    let subject = file.display().to_string();
    let mut command = tools.ffmpeg();
    command
        .args(["-nostdin", "-v", "error"])
        .arg("-i")
        .arg(file)
        .args(["-vn", "-ar", &ANALYSIS_RATE.to_string()])
        .args(["-ac", "1", "-f", SAMPLE_FORMAT, "-"])
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

    let mut profiler = Profiler::sectioned(1, ANALYSIS_RATE, sections);
    read_into(&mut stdout, &mut profiler)?;
    // Waited on only after the pipe has run dry: ffmpeg blocks writing into a
    // full pipe, so a process we stopped reading from would never exit.
    crate::pipe::finish(child, Stage::Measure, &subject)?;
    Ok(profiler.finish())
}

/// Reads every sample ffmpeg produces into the profiler.
///
/// A word can straddle two reads — a pipe is free to hand back any number of
/// bytes — so the remainder of an incomplete one is carried into the next
/// chunk. Dropping it instead would shift every later sample by up to three
/// bytes, which is not a rounding error but a different signal.
fn read_into(source: &mut impl Read, profiler: &mut Profiler) -> Result<(), RenderError> {
    let word = size_of::<f32>();
    let mut bytes = vec![0_u8; CHUNK];
    let mut carried = 0;
    let mut samples = Vec::with_capacity(CHUNK / 4);
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
        let whole = filled / word * word;
        samples.clear();
        samples.extend(
            bytes[..whole]
                .chunks_exact(word)
                .map(|word| f32::from_le_bytes(word.try_into().expect("four bytes"))),
        );
        profiler.feed(&samples);
        bytes.copy_within(whole..filled, 0);
        carried = filled - whole;
    }
}
