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
//! `.wav`: the analysis never learns what container it came out of. The decode
//! itself is [`super::read`]'s, which is where the reason a file is metered at
//! its own channel count is written down.
//!
//! **One table, whatever the file has in it.** The report is about the file and
//! not about its channels: loudness and spectral balance are answered for the
//! whole of it at once, and measuring the channels separately would produce two
//! tables to read where the finding — this stretch is quiet, this one is
//! muddy — is the same in both.

use std::path::Path;

use scorsese_zimmer::level::{Cut, Profile, Profiler};

use crate::error::RenderError;
use crate::tools::Tools;

use super::read::{self, ANALYSIS_RATE};

/// How `file` came out, over its whole length and section by section.
pub fn measure(tools: &Tools, file: &Path) -> Result<Profile, RenderError> {
    profile(tools, file, Vec::new())
}

/// How `file` came out, cut where `sections` says rather than on the clock.
///
/// For a bake whose recipe is still in hand: the arrangement knows where its
/// patterns are, and a row that names one is worth several that name a time.
pub(crate) fn profile(
    tools: &Tools,
    file: &Path,
    sections: Vec<Cut>,
) -> Result<Profile, RenderError> {
    // The count reaches the profiler and ffmpeg from one place, so the grid the
    // sections are cut on and the grid the samples arrive on cannot disagree.
    let channels = read::channels(tools, file);
    let mut profiler = Profiler::sectioned(channels, ANALYSIS_RATE, sections);
    read::decode(tools, file, channels, |samples| profiler.feed(samples))?;
    Ok(profiler.finish())
}
