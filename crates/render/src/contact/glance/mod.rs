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
//!   middle of its slice of the file, a few decoded at once; one that decodes
//!   nothing is skipped.
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

mod signature;
mod transcript;

use signature::Signature;
use transcript::transcript;

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

/// How many candidates are decoded at once: each is its own ffmpeg seek, and
/// one after another a long file's two dozen take seconds.
const DECODES_AT_ONCE: usize = 4;

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
/// than [`GLANCE_FEWEST`] for every 15 seconds, up to
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

    let frames = decoded(
        tools,
        file,
        &candidates(duration_seconds, count),
        resolution,
    )?;
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

/// Each of `moments` decoded at `resolution`, [`DECODES_AT_ONCE`] at a time,
/// in order. A moment that decodes nothing — past where the picture ends in
/// a file whose sound runs on — is left out; only when none decode is the
/// first one's error the answer.
fn decoded(
    tools: &Tools,
    file: &Path,
    moments: &[f64],
    resolution: scorsese_compositor::Resolution,
) -> Result<Vec<(f64, Frame)>, ContactError> {
    let mut results = Vec::with_capacity(moments.len());
    for batch in moments.chunks(DECODES_AT_ONCE) {
        std::thread::scope(|scope| {
            let running: Vec<_> = batch
                .iter()
                .map(|&at| scope.spawn(move || (at, decode(tools, file, at, resolution))))
                .collect();
            for one in running {
                results.push(one.join().expect("decoding a frame does not panic"));
            }
        });
    }
    let mut first_error = None;
    let mut frames = Vec::with_capacity(results.len());
    for (at, result) in results {
        match result {
            Ok(frame) => frames.push((at, frame)),
            Err(error) => {
                first_error.get_or_insert(error);
            }
        }
    }
    match first_error {
        Some(error) if frames.is_empty() => Err(error),
        _ => Ok(frames),
    }
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

#[cfg(test)]
mod tests {
    use super::signature::tests::{busy, flat};
    use super::*;

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
        assert!(
            short.iter().all(|at| *at <= 0.15),
            "none past the last frame of a fifth of a second: {short:?}"
        );
    }

    #[test]
    fn a_length_not_known_is_the_floor_and_the_first_moment() {
        assert_eq!(glance_count(f64::NAN), GLANCE_FEWEST);
        assert_eq!(glance_count(-5.0), GLANCE_FEWEST);
        assert_eq!(glance_count(f64::INFINITY), GLANCE_FEWEST);
        assert_eq!(candidates(f64::NAN, 3), [0.0]);
        assert_eq!(candidates(-1.0, 3), [0.0]);
    }

    #[test]
    fn evenly_keeps_both_ends() {
        assert_eq!(evenly(&[1, 2, 3, 4, 5], 3), [1, 3, 5]);
        assert_eq!(evenly(&[4, 9], 1), [9]);
        assert_eq!(evenly(&[4, 9], 0), Vec::<usize>::new());
    }
}
