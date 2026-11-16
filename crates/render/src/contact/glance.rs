//! A glance at a whole video: what a model is shown of it (#1563).
//!
//! [`super::sheet`] takes the moments a caller names. This chooses them, so
//! that everything showing a video to a model — `look` asked for nothing in
//! particular, a video attached to a message on the web — shows the same
//! frames, chosen the same way:
//!
//! - **The count follows the length** ([`glance_count`]): between
//!   [`GLANCE_FEWEST`] and [`GLANCE_MOST`].
//! - **Candidates are sampled evenly**, twice as many as are kept, each in the
//!   middle of its slice of the file.
//! - **A frame that shows nothing is dropped**: black, a flat colour, the
//!   bottom of a fade.
//! - **A frame nearly identical to the last one kept is dropped**, so a video
//!   that barely changes earns the floor, not the count its length is worth.
//! - The survivors are thinned evenly to the count, or topped up to the floor
//!   from what was dropped (repeats before blanks) when too few survive.
//!
//! Both drops compare a [`Signature`]: the frame's brightness averaged over a
//! coarse grid, which ignores codec noise and keeps what a viewer would call
//! the picture. When the file has a transcript, its words come back beside the
//! sheet ([`Glance::transcript`]), each line led by the time it is said in the
//! same seconds the cells are labelled with. Nothing here ever starts one.

use std::path::Path;

use scorsese_compositor::Frame;
use scorsese_compositor::sheet::{self, Cell, MAX_CELLS};
use scorsese_compositor::text::Font;
use scorsese_core::words::Words;

use super::sample::{self, label};
use super::{ContactError, Sheet, decode};
use crate::probe::Ffprobe;
use crate::tools::Tools;
use scorsese_core::probe::ProbeMedia;

/// The fewest frames a glance shows, however short or still the video.
pub const GLANCE_FEWEST: usize = 3;

/// The most frames a glance shows: a whole sheet.
pub const GLANCE_MOST: usize = MAX_CELLS;

/// Seconds of video that earn one frame past [`GLANCE_FEWEST`]: a minute is
/// seven frames, three minutes and more the whole [`GLANCE_MOST`].
const SECONDS_PER_FRAME: f64 = 15.0;

/// How many candidates are sampled for each frame kept.
const CANDIDATES_PER_FRAME: usize = 2;

/// Cells across and down a [`Signature`].
const SIGNATURE_SIDE: usize = 16;

/// A frame whose cells' brightness spreads less than this (a standard
/// deviation, on 0–255) shows nothing: black, a flat colour, a fade's bottom.
const BLANK_SPREAD: f64 = 6.0;

/// A frame whose cells differ from the last kept one's by less than this on
/// average (on 0–255) is a near-repeat of it.
const NEAR_REPEAT: f64 = 4.0;

/// A glance: the sheet, and what is said in the file when it has a transcript.
#[derive(Debug, Clone, PartialEq)]
pub struct Glance {
    /// The chosen frames, tiled and labelled with their times.
    pub sheet: Sheet,
    /// The file's words, one line per phrase, each led by the time it starts
    /// (`0:12 Today we cook.`) — `None` when no words were given, or none
    /// were in them.
    pub transcript: Option<String>,
}

/// How many frames a glance at something `seconds` long shows: one more
/// than [`GLANCE_FEWEST`] for every [`SECONDS_PER_FRAME`], up to
/// [`GLANCE_MOST`]. A length that is not known is the floor.
pub fn glance_count(seconds: f64) -> usize {
    if !seconds.is_finite() || seconds <= 0.0 {
        return GLANCE_FEWEST;
    }
    let more = (seconds / SECONDS_PER_FRAME).floor() as usize;
    (GLANCE_FEWEST + more).min(GLANCE_MOST)
}

/// A glance at `file`, with `words` as its transcript when it has one —
/// the caller's to find, since a project keeps it beside the file and the
/// web on the library's row. `ruled` draws the coordinate grid over each
/// cell, as [`super::Look::grid`] does.
pub fn glance(
    tools: &Tools,
    file: &Path,
    words: Option<&Words>,
    ruled: bool,
) -> Result<Glance, ContactError> {
    let media = Ffprobe::new(tools.clone()).probe(file)?;
    let (Some(width), Some(height)) = (media.width, media.height) else {
        return Err(ContactError::NoPicture {
            file: file.to_path_buf(),
        });
    };
    let duration_seconds = media.duration_seconds.unwrap_or_default();
    let count = glance_count(duration_seconds);
    let (cell_width, cell_height) = sample::cell(width, height);
    let resolution =
        scorsese_compositor::Resolution::new(cell_width, cell_height).map_err(|source| {
            ContactError::Cell {
                file: file.to_path_buf(),
                source,
            }
        })?;

    let mut frames = Vec::new();
    for at in candidates(duration_seconds, count) {
        frames.push((at, decode(tools, file, at, resolution)?));
    }
    let signatures: Vec<Signature> = frames
        .iter()
        .map(|(_, frame)| Signature::of(frame))
        .collect();
    let kept = choose(&signatures, count);

    let mut at_seconds = Vec::with_capacity(kept.len());
    let mut cells = Vec::with_capacity(kept.len());
    let mut frames: Vec<Option<(f64, Frame)>> = frames.into_iter().map(Some).collect();
    for index in kept {
        let (at, frame) = frames[index].take().expect("each candidate is kept once");
        at_seconds.push(at);
        cells.push(Cell {
            frame,
            label: label(at),
        });
    }
    Ok(Glance {
        sheet: Sheet {
            image: sheet::tile(cells, Font::sans(), ruled)?,
            at_seconds,
            duration_seconds,
            file: file.to_path_buf(),
        },
        transcript: words.and_then(transcript),
    })
}

/// The moments sampled from something `seconds` long for a glance of
/// `count`: [`CANDIDATES_PER_FRAME`] for each, evenly, each in the middle of
/// its slice and rounded to a tenth, none past the last frame. A length that
/// is not known is the first moment alone.
fn candidates(seconds: f64, count: usize) -> Vec<f64> {
    if !seconds.is_finite() || seconds <= 0.0 {
        return vec![0.0];
    }
    let last = (seconds - sample::LAST_FRAME_MARGIN).max(0.0);
    let slices = count * CANDIDATES_PER_FRAME;
    let mut moments: Vec<f64> = (0..slices)
        .map(|slice| {
            let middle = seconds * (slice as f64 + 0.5) / slices as f64;
            sample::tenths_within(middle.min(last), last)
        })
        .collect();
    moments.dedup();
    moments
}

/// Which of the candidates, by index and in order, a glance of `count` keeps.
fn choose(signatures: &[Signature], count: usize) -> Vec<usize> {
    let mut kept: Vec<usize> = Vec::new();
    for (index, signature) in signatures.iter().enumerate() {
        if signature.blank() {
            continue;
        }
        let repeat = kept
            .last()
            .is_some_and(|&last| signature.difference(&signatures[last]) < NEAR_REPEAT);
        if !repeat {
            kept.push(index);
        }
    }
    if kept.len() > count {
        kept = evenly(&kept, count);
    }
    let floor = GLANCE_FEWEST.min(signatures.len());
    if kept.len() < floor {
        let unkept = |blank: bool| -> Vec<usize> {
            (0..signatures.len())
                .filter(|index| !kept.contains(index) && signatures[*index].blank() == blank)
                .collect()
        };
        for pool in [unkept(false), unkept(true)] {
            let wanted = floor - kept.len();
            kept.extend(evenly(&pool, wanted.min(pool.len())));
        }
        kept.sort_unstable();
    }
    kept
}

/// `count` of `from`, spread evenly from its first to its last.
fn evenly(from: &[usize], count: usize) -> Vec<usize> {
    match count {
        0 => Vec::new(),
        1 => from.get(from.len() / 2).copied().into_iter().collect(),
        _ => (0..count)
            .map(|n| from[(n * (from.len() - 1) + (count - 1) / 2) / (count - 1)])
            .collect(),
    }
}

/// A frame's brightness averaged over a [`SIGNATURE_SIDE`]-square grid of
/// cells: what two frames are compared by, and a blank one is told by.
#[derive(Debug, Clone, PartialEq)]
struct Signature([f64; SIGNATURE_SIDE * SIGNATURE_SIDE]);

impl Signature {
    fn of(frame: &Frame) -> Self {
        let width = frame.resolution().width() as usize;
        let height = frame.resolution().height() as usize;
        let mut sums = [0.0; SIGNATURE_SIDE * SIGNATURE_SIDE];
        let mut counts = [0_u32; SIGNATURE_SIDE * SIGNATURE_SIDE];
        for (pixel, rgba) in frame.bytes().chunks_exact(4).enumerate() {
            let (x, y) = (pixel % width, pixel / width);
            let cell = (y * SIGNATURE_SIDE / height) * SIGNATURE_SIDE + x * SIGNATURE_SIDE / width;
            sums[cell] += 0.299 * f64::from(rgba[0])
                + 0.587 * f64::from(rgba[1])
                + 0.114 * f64::from(rgba[2]);
            counts[cell] += 1;
        }
        for (sum, count) in sums.iter_mut().zip(counts) {
            *sum /= f64::from(count.max(1));
        }
        Self(sums)
    }

    /// Whether it shows nothing: its cells' brightness hardly spreads.
    fn blank(&self) -> bool {
        let mean = self.0.iter().sum::<f64>() / self.0.len() as f64;
        let variance =
            self.0.iter().map(|cell| (cell - mean).powi(2)).sum::<f64>() / self.0.len() as f64;
        variance.sqrt() < BLANK_SPREAD
    }

    /// How far apart two frames are: their cells' mean absolute difference.
    fn difference(&self, other: &Self) -> f64 {
        self.0
            .iter()
            .zip(other.0.iter())
            .map(|(a, b)| (a - b).abs())
            .sum::<f64>()
            / self.0.len() as f64
    }
}

/// The most words one line of a transcript holds before it is broken.
const WORDS_PER_LINE: usize = 16;

/// A pause this long, in seconds, between two words starts a new line.
const LINE_PAUSE: f64 = 1.0;

/// `words` as lines, each led by when its first word starts: broken after a
/// sentence ends, at a pause of [`LINE_PAUSE`], or at [`WORDS_PER_LINE`].
/// `None` when there are no words.
fn transcript(words: &Words) -> Option<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line: Vec<&str> = Vec::new();
    let mut started = 0.0;
    let mut previous_end: Option<f64> = None;
    for word in &words.words {
        let paused = previous_end.is_some_and(|end| word.start - end >= LINE_PAUSE);
        if paused && !line.is_empty() {
            lines.push(said_at(started, &line));
            line.clear();
        }
        if line.is_empty() {
            started = word.start;
        }
        line.push(&word.text);
        previous_end = Some(word.end);
        let ended = word.text.ends_with(['.', '?', '!', '…']);
        if ended || line.len() >= WORDS_PER_LINE {
            lines.push(said_at(started, &line));
            line.clear();
        }
    }
    if !line.is_empty() {
        lines.push(said_at(started, &line));
    }
    (!lines.is_empty()).then(|| lines.join("\n"))
}

/// One line of a transcript: when it starts, as a cell's label reads, then
/// its words.
fn said_at(seconds: f64, words: &[&str]) -> String {
    format!("{} {}", label(sample::tenths(seconds)), words.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(level: f64) -> Signature {
        Signature([level; SIGNATURE_SIDE * SIGNATURE_SIDE])
    }

    /// A busy signature, one of its own for each `which`.
    fn busy(which: usize) -> Signature {
        let mut cells = [0.0; SIGNATURE_SIDE * SIGNATURE_SIDE];
        for (index, cell) in cells.iter_mut().enumerate() {
            *cell = 30.0 + ((index * 7 + which * 37) % 190) as f64;
        }
        Signature(cells)
    }

    #[test]
    fn a_flat_frame_is_blank_and_a_busy_one_is_not() {
        assert!(flat(0.0).blank());
        assert!(flat(200.0).blank());
        assert!(!busy(0).blank());
    }

    #[test]
    fn blanks_and_repeats_are_dropped_and_the_rest_kept_in_order() {
        let signatures = [flat(0.0), busy(0), busy(0), busy(1), flat(0.0), busy(2)];
        assert_eq!(choose(&signatures, 12), [1, 3, 5]);
    }

    #[test]
    fn too_many_survivors_are_thinned_evenly_keeping_both_ends() {
        let signatures: Vec<Signature> = (0..9).map(busy).collect();
        let kept = choose(&signatures, 5);
        assert_eq!(kept.len(), 5);
        assert_eq!(kept, [0, 2, 4, 6, 8]);
    }

    #[test]
    fn too_few_are_topped_up_from_repeats_before_blanks() {
        let signatures = [flat(0.0), busy(0), busy(0), busy(0), flat(0.0)];
        let kept = choose(&signatures, 3);
        assert_eq!(kept.len(), 3);
        assert!(kept.iter().all(|index| (1..=3).contains(index)), "{kept:?}");
        let blank = [flat(0.0), flat(0.0), flat(0.0), flat(0.0)];
        assert_eq!(choose(&blank, 3).len(), 3);
        assert_eq!(choose(&[busy(0)], 3), [0], "never more than there are");
    }

    #[test]
    fn candidates_are_spread_in_the_middle_of_their_slices() {
        assert_eq!(candidates(30.0, 3), [2.5, 7.5, 12.5, 17.5, 22.5, 27.5]);
        assert_eq!(candidates(0.0, 3), [0.0]);
        let short = candidates(0.2, 3);
        assert!(short.windows(2).all(|pair| pair[0] < pair[1]), "{short:?}");
    }

    #[test]
    fn evenly_keeps_both_ends() {
        assert_eq!(evenly(&[1, 2, 3, 4, 5], 3), [1, 3, 5]);
        assert_eq!(evenly(&[4, 9], 1), [9]);
        assert_eq!(evenly(&[4, 9], 0), Vec::<usize>::new());
    }

    #[test]
    fn a_transcript_breaks_at_sentences_and_pauses() {
        let word = |text: &str, start: f64| scorsese_core::words::Word {
            text: text.to_owned(),
            start,
            end: start + 0.3,
        };
        let words = Words {
            words: vec![
                word("So", 0.0),
                word("then", 0.4),
                word("later", 5.0),
                word("on.", 5.4),
                word("Done", 6.0),
            ],
        };
        assert_eq!(
            transcript(&words).as_deref(),
            Some("0:00 So then\n0:05 later on.\n0:06 Done")
        );
    }
}
