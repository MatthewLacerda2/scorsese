//! Turning a sound file into a picture of itself.
//!
//! The audio half of [`crate::contact`]. That one exists because an assistant
//! handed twenty clips is arranging file names and hoping; this one exists
//! because an assistant handed a narration file is comparing byte hashes and
//! hoping. Three lines were generated against a live API and verified by file
//! size, hash and a probed duration of 3.67 seconds — every one of which is
//! equally consistent with 3.67 seconds of silence.
//!
//! ffmpeg decodes and we do the rest, as everywhere else in this crate, which
//! is also what makes this work on a delivered `.mp4` as readily as on the
//! `.mp3` a provider returned: the analysis never learns what container it came
//! out of.
//!
//! The picture is drawn by `scorsese-compositor`, because compositing is ours,
//! and the samples arrive through [`super::read`], which is the same decode the
//! level report reads a file with — so the picture and the numbers beside it
//! cannot disagree about what is in a file. What this module owns is the
//! slicing and the handful of facts that go back **in words** beside the
//! picture: a client that cannot see images still has to get a useful answer,
//! which is the rule the other picture-returning tools already follow.

use std::path::{Path, PathBuf};

use scorsese_compositor::text::Font;
use scorsese_compositor::waveform::{self, COLUMNS, Column, Wave};
use scorsese_compositor::{Frame, ResolutionError};

use crate::error::RenderError;
use crate::tools::Tools;

use super::read::{self, ANALYSIS_RATE};

/// Below this the picture and the report both call a slice silent.
///
/// −60 dBFS. Not zero: an encoder's dither and a room's noise floor both land
/// a few counts above it, and a threshold of exactly zero would report a file
/// with a gap in it as having none.
const SILENCE: f32 = 0.001;

/// What the columns say about the file, in numbers.
///
/// Worked out from the **columns** rather than from the samples a second time,
/// so the picture and the words cannot disagree about what is in the file —
/// which is the one failure that would make both of them untrustworthy.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Findings {
    /// The loudest sample anywhere in the file, as an absolute amplitude.
    pub peak: f32,
    /// The root-mean-square of the whole file.
    pub rms: f32,
    /// How many slices reached full scale. Non-zero is clipping.
    pub clipped: usize,
    /// Silence before the first sound, in seconds.
    pub lead_in: f64,
    /// Silence after the last, in seconds.
    pub lead_out: f64,
}

/// A picture of a sound file, and what it is a picture of.
#[derive(Debug, Clone, PartialEq)]
pub struct Waveform {
    /// The drawn waveform.
    pub image: Frame,
    /// The file this is a picture of.
    pub file: PathBuf,
    /// How long it is, in seconds.
    pub seconds: f64,
    /// What the picture shows, as numbers a client with no vision can read.
    pub findings: Findings,
}

impl Findings {
    /// True when nothing anywhere in the file rose above the silence floor.
    ///
    /// The finding this whole module exists for. A file that is silent has a
    /// duration, a size and a hash exactly like one that is not.
    pub fn is_silent(&self) -> bool {
        self.peak < SILENCE
    }

    /// The peak as dBFS, or `None` for a file with no signal in it at all —
    /// where the honest answer is "silent" rather than a large negative number.
    pub fn peak_dbfs(&self) -> Option<f64> {
        (!self.is_silent()).then(|| 20.0 * f64::from(self.peak).log10())
    }
}

/// Decodes `file`, slices it into columns, and draws the waveform.
///
/// The face is chosen here rather than asked of the caller, exactly as a
/// contact sheet's is: which font a label is set in is this crate's business,
/// and a caller that had to name one would be a caller that could name the
/// wrong one.
pub fn waveform(tools: &Tools, file: &Path) -> Result<Waveform, WaveError> {
    let channels = read::channels(tools, file);
    let mut samples = Vec::new();
    read::decode(tools, file, channels, |run| samples.extend_from_slice(run))?;
    let seconds = (samples.len() / channels) as f64 / f64::from(ANALYSIS_RATE);
    let columns = slice(&samples, channels);
    let findings = summarise(&columns, seconds);
    let image = waveform::draw(
        &Wave {
            columns: &columns,
            seconds,
            caption: &caption(file, seconds, findings),
        },
        Font::sans(),
    )?;
    Ok(Waveform {
        image,
        file: file.to_path_buf(),
        seconds,
        findings,
    })
}

/// Why a waveform could not be made.
#[derive(Debug, thiserror::Error)]
pub enum WaveError {
    /// ffmpeg could not decode the file.
    #[error(transparent)]
    Decode(#[from] RenderError),
    /// The picture's own size was refused, which would mean the constants in
    /// the compositor had been edited into something impossible.
    #[error("the waveform picture is not a usable raster: {0}")]
    Raster(#[from] ResolutionError),
}

/// Cuts the samples into one slice per pixel column.
///
/// A short file gets fewer columns rather than a stretched picture: one column
/// per pixel is the most faithful summary there is, and inventing columns a
/// file has no samples for would draw detail that is not in it.
///
/// **One picture for the whole file**, whatever it has in it: a column's peak
/// is the loudest sample in any channel over that stretch and its level is the
/// mean square of all of them, so the drawn envelope is the file's own and a
/// clip mark means the file clipped. Two waveforms to read would be two
/// pictures where one answers — and cutting the columns on **frames** is what
/// keeps a column from holding one channel of an instant without the other.
fn slice(samples: &[f32], channels: usize) -> Vec<Column> {
    let frames = samples.len() / channels;
    if frames == 0 {
        return Vec::new();
    }
    let per = frames.div_ceil(COLUMNS).max(1);
    samples[..frames * channels]
        .chunks(per * channels)
        .map(|slice| {
            let peak = slice.iter().fold(0.0_f32, |most, one| most.max(one.abs()));
            let sum: f64 = slice
                .iter()
                .map(|one| f64::from(*one) * f64::from(*one))
                .sum();
            Column {
                peak,
                rms: (sum / slice.len() as f64).sqrt() as f32,
            }
        })
        .collect()
}

/// The findings for a set of columns.
fn summarise(columns: &[Column], seconds: f64) -> Findings {
    let peak = columns.iter().fold(0.0_f32, |most, one| most.max(one.peak));
    let sum: f64 = columns
        .iter()
        .map(|one| f64::from(one.rms) * f64::from(one.rms))
        .sum();
    let per_column = if columns.is_empty() {
        0.0
    } else {
        seconds / columns.len() as f64
    };
    let sounding = |one: &Column| one.peak >= SILENCE;
    Findings {
        peak,
        rms: if columns.is_empty() {
            0.0
        } else {
            (sum / columns.len() as f64).sqrt() as f32
        },
        clipped: columns.iter().filter(|one| one.peak >= 1.0).count(),
        lead_in: columns.iter().position(sounding).unwrap_or(columns.len()) as f64 * per_column,
        lead_out: columns.iter().rev().position(sounding).unwrap_or(0) as f64 * per_column,
    }
}

/// The line written along the top of the picture.
///
/// On the picture as well as in the reply, because a saved PNG travels away
/// from whatever printed the reply — `scorsese hear` writes a file somebody
/// opens later, and a waveform with no scale on it is a shape.
fn caption(file: &Path, seconds: f64, findings: Findings) -> String {
    let name = file.file_name().map_or_else(
        || file.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    let level = match findings.peak_dbfs() {
        None => "silent throughout".to_owned(),
        Some(db) => format!("peak {db:.1} dBFS"),
    };
    let clipping = if findings.clipped > 0 {
        format!(", {} clipped", findings.clipped)
    } else {
        String::new()
    };
    format!("{name} — {seconds:.2}s, {level}{clipping}")
}
