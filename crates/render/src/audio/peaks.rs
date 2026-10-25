//! A sound's peaks: the levels a timeline draws a clip's waveform from.
//!
//! The web app's timeline draws the slice of a sound a clip plays, at any trim
//! and any zoom, from one small array per file fetched once (#1572). A picture
//! of the whole file cannot be cut to a trim or stretched to a zoom; a level
//! per stretch of time can. ffmpeg decodes, through the same reader the level
//! report uses, and the arithmetic is ours.

use std::path::Path;

use crate::error::RenderError;
use crate::tools::Tools;

use super::read::{self, ANALYSIS_RATE};

/// How many levels a second of sound is read as.
///
/// The timeline's closest zoom is 320 pixels a second, so forty puts a level
/// every eight pixels there: a bar with a gap beside it, as editors draw one.
/// A three-minute file is 7,204 bytes on the wire ([`Peaks::to_bytes`]).
pub const PEAKS_PER_SECOND: u32 = 40;

/// The most levels one file is read as. Past it (13.6 minutes at
/// [`PEAKS_PER_SECOND`]), runs of neighbours are merged and the rate falls:
/// an hour-long file is merged in fives, 8 levels a second, 28,804 bytes.
pub const MOST_PEAKS: usize = 32_768;

/// A sound's levels over time.
#[derive(Debug, Clone, PartialEq)]
pub struct Peaks {
    /// How many levels stand for one second of the file.
    pub per_second: f32,
    /// The loudest sample of each stretch, any channel, on a byte's linear
    /// scale: 0 is silence, 255 full scale or past it.
    pub levels: Vec<u8>,
}

impl Peaks {
    /// The stored and sent form: [`Peaks::per_second`] as a little-endian
    /// `f32`, then one byte a level.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.per_second.to_le_bytes().to_vec();
        bytes.extend_from_slice(&self.levels);
        bytes
    }

    /// The peaks [`Peaks::to_bytes`] wrote; `None` for fewer than four bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let (rate, levels) = bytes.split_first_chunk::<4>()?;
        Some(Self {
            per_second: f32::from_le_bytes(*rate),
            levels: levels.to_vec(),
        })
    }

    /// At most `most` levels: each run of neighbours merged into its loudest,
    /// and the rate divided by the run's length.
    pub fn fit(self, most: usize) -> Self {
        let run = self.levels.len().div_ceil(most.max(1)).max(1);
        if run == 1 {
            return self;
        }
        Self {
            per_second: self.per_second / run as f32,
            levels: self
                .levels
                .chunks(run)
                .map(|merged| merged.iter().copied().max().unwrap_or(0))
                .collect(),
        }
    }
}

/// Decodes `file` and reads it as [`PEAKS_PER_SECOND`] levels a second, at
/// most [`MOST_PEAKS`] of them.
pub fn peaks(tools: &Tools, file: &Path) -> Result<Peaks, RenderError> {
    let channels = read::channels(tools, file);
    let window = (ANALYSIS_RATE / PEAKS_PER_SECOND) as usize * channels;
    let mut levels = Vec::new();
    let (mut loudest, mut taken) = (0.0_f32, 0);
    read::decode(tools, file, channels, |run| {
        for sample in run {
            loudest = loudest.max(sample.abs());
            taken += 1;
            if taken == window {
                levels.push(level(loudest));
                (loudest, taken) = (0.0, 0);
            }
        }
    })?;
    if taken > 0 {
        levels.push(level(loudest));
    }
    Ok(Peaks {
        per_second: PEAKS_PER_SECOND as f32,
        levels,
    }
    .fit(MOST_PEAKS))
}

/// An absolute amplitude on a byte's scale.
fn level(peak: f32) -> u8 {
    (peak.min(1.0) * 255.0).round() as u8
}
