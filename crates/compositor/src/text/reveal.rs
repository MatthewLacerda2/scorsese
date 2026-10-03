//! A block of text arriving a piece at a time.
//!
//! **The layout does not move.** Every piece is set exactly where the whole
//! block would set it — same wrapping, same kerning, same line — and a reveal
//! only decides how solid each piece is and how far below its place it sits at
//! this instant. So the words of a half-revealed caption are where they will be
//! when it has finished, and nothing reflows as the rest arrive.
//!
//! **Pieces are clusters, words or lines, in reading order.** A cluster is
//! [`super::runs::clusters`]'s, so an emoji with a skin tone, a flag or an
//! accented letter is one piece and never arrives in halves; a glyph a
//! fallback face drew belongs to its piece like any other. Whitespace is never
//! a piece: a space has nothing to fade in, and counting one would put a pause
//! in the sweep where nothing visible happens.

use std::ops::Range;

use scorsese_core::{Easing, RevealUnit};

use super::layout::Line;
use super::runs;
use super::shape::holds;

/// Where the reveal is at one instant, as the `reveal` property resolved it.
///
/// **Linear, with the easing carried beside it** rather than applied. The
/// sweep across the pieces is even; the easing shapes each piece's own
/// entrance, which is where an overshoot reads as a word landing rather than
/// as the whole line lurching.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sweep {
    /// How far through the pieces, `0.0` none and `1.0` all, linear in time.
    pub at: f64,
    /// What each piece's own entrance is eased with.
    pub easing: Easing,
    /// Whether the sweep is running from shown towards hidden — an exit. The
    /// easing then applies in that direction of time, so a `back_in` exit
    /// winds up before it leaves exactly as it would on any other property.
    pub backwards: bool,
}

impl Sweep {
    /// Everything shown and nothing moving: what a text with no `reveal`
    /// track is.
    pub const DONE: Self = Self {
        at: 1.0,
        easing: Easing::Linear,
        backwards: false,
    };
}

/// How a block reveals, in pixels of the raster it is drawn on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reveal {
    /// What the text is cut into.
    pub unit: RevealUnit,
    /// How far below its place a piece starts, in pixels. Negative is above.
    pub rise: f32,
    /// How far one piece gets through its entrance before the next starts,
    /// `0.0` to `1.0`.
    pub stagger: f64,
    /// Where the reveal is.
    pub sweep: Sweep,
}

/// How one piece is drawn at this instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Shown {
    /// How solid, `0.0` to `1.0` — never past either, whatever the easing does.
    pub opacity: f32,
    /// How far down the raster from its place, in pixels. An overshooting
    /// easing takes this past zero, which is the piece rising above its line
    /// and settling back.
    pub drop: f32,
}

impl Reveal {
    /// How piece `index` of `count` is drawn.
    pub(super) fn shown(&self, index: usize, count: usize) -> Shown {
        let stagger = self.stagger.clamp(0.0, 1.0);
        // Each piece's entrance is `ramp` of the sweep, and the next one starts
        // `stagger` of the way through it — so the last one finishes exactly at
        // the sweep's end, whatever the count and the overlap.
        let ramp = 1.0 / (1.0 + count.saturating_sub(1) as f64 * stagger);
        let start = index as f64 * stagger * ramp;
        // Snapped at both ends, so a finished sweep leaves every piece exactly
        // at rest rather than a rounding error below its line.
        const SNAP: f64 = 1e-9;
        let local = match (self.sweep.at - start) / ramp {
            local if local >= 1.0 - SNAP => 1.0,
            local if local <= SNAP => 0.0,
            local => local,
        };
        let eased = if self.sweep.backwards {
            1.0 - self.sweep.easing.apply(1.0 - local)
        } else {
            self.sweep.easing.apply(local)
        };
        Shown {
            opacity: eased.clamp(0.0, 1.0) as f32,
            drop: self.rise * (1.0 - eased) as f32,
        }
    }
}

/// Every piece of a laid-out block, found by line and by byte offset.
pub(super) struct Pieces {
    /// Per line, the byte ranges of its pieces in order.
    lines: Vec<Vec<Range<usize>>>,
    /// Per line, the index its first piece has across the whole block.
    first: Vec<usize>,
    /// How many pieces the block has.
    count: usize,
}

impl Pieces {
    /// The pieces `lines` are cut into.
    pub(super) fn of(lines: &[Line], unit: RevealUnit) -> Self {
        let mut pieces = Self {
            lines: Vec::with_capacity(lines.len()),
            first: Vec::with_capacity(lines.len()),
            count: 0,
        };
        for line in lines {
            let ranges = cut(&line.text, unit);
            pieces.first.push(pieces.count);
            pieces.count += ranges.len();
            pieces.lines.push(ranges);
        }
        pieces
    }

    /// How many there are.
    pub(super) fn count(&self) -> usize {
        self.count
    }

    /// Which piece the character at byte `cluster` of line `row` belongs to,
    /// or `None` for whitespace, which belongs to none.
    pub(super) fn index(&self, row: usize, cluster: usize) -> Option<usize> {
        let ranges = self.lines.get(row)?;
        let at = ranges.iter().position(|range| range.contains(&cluster))?;
        Some(self.first[row] + at)
    }
}

/// One line's pieces, as byte ranges of its text.
fn cut(text: &str, unit: RevealUnit) -> Vec<Range<usize>> {
    let blank = |range: &Range<usize>| text[range.clone()].chars().all(char::is_whitespace);
    let ranges = match unit {
        RevealUnit::Char => runs::clusters(text),
        RevealUnit::Word => words(text),
        RevealUnit::Line => std::iter::once(0..text.len()).collect(),
    };
    ranges.into_iter().filter(|range| !blank(range)).collect()
}

/// The words of a line: stretches between the spaces it could break at.
///
/// The same notion of a word the line breaker uses, so a held space — a
/// non-breaking one, or the padding of a counting number — keeps the two sides
/// of it one word here as it does there.
fn words(text: &str) -> Vec<Range<usize>> {
    let mut found = Vec::new();
    let mut start = None;
    for (at, character) in text.char_indices() {
        let breaks = character.is_whitespace() && !holds(character);
        match (breaks, start) {
            (true, Some(from)) => {
                found.push(from..at);
                start = None;
            }
            (false, None) => start = Some(at),
            _ => {}
        }
    }
    if let Some(from) = start {
        found.push(from..text.len());
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reveal(at: f64, stagger: f64) -> Reveal {
        Reveal {
            unit: RevealUnit::Word,
            rise: 10.0,
            stagger,
            sweep: Sweep { at, ..Sweep::DONE },
        }
    }

    #[test]
    fn nothing_shows_at_the_start_and_everything_rests_at_the_end() {
        for index in 0..4 {
            assert_eq!(reveal(0.0, 0.5).shown(index, 4).opacity, 0.0);
            let done = reveal(1.0, 0.5).shown(index, 4);
            assert_eq!((done.opacity, done.drop), (1.0, 0.0));
        }
    }

    /// A stagger of one is a queue: at the halfway mark of four pieces, the
    /// first two are in and the last two have not begun.
    #[test]
    fn a_full_stagger_takes_the_pieces_one_after_another() {
        let halfway = reveal(0.5, 1.0);
        let opacities: Vec<f32> = (0..4).map(|i| halfway.shown(i, 4).opacity).collect();
        assert_eq!(opacities, vec![1.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn no_stagger_is_every_piece_at_once() {
        let midway = reveal(0.25, 0.0);
        assert!((0..4).all(|i| midway.shown(i, 4) == midway.shown(0, 4)));
    }

    /// An overshooting easing lifts a piece past its line — and never makes it
    /// more than solid.
    #[test]
    fn an_overshoot_moves_the_piece_and_not_its_opacity() {
        let popping = Reveal {
            sweep: Sweep {
                at: 0.7,
                easing: Easing::BackOut,
                backwards: false,
            },
            ..reveal(0.0, 0.0)
        };
        let shown = popping.shown(0, 1);
        assert!(shown.drop < 0.0, "above its line: {}", shown.drop);
        assert_eq!(shown.opacity, 1.0);
    }

    /// A sweep a hair past a piece's start — closer than rounding can tell
    /// from it — has not begun it: nothing showing, the whole rise below.
    #[test]
    fn a_piece_a_rounding_error_in_has_not_begun() {
        let shown = reveal(5e-10, 0.0).shown(0, 1);
        assert_eq!((shown.opacity, shown.drop), (0.0, 10.0));
    }

    /// An exit runs the entrance in reverse: the piece leaving is where the
    /// same easing would have brought it arriving from the other end — so an
    /// ease-in exit halfway out still shows three quarters, not a quarter.
    #[test]
    fn a_backwards_sweep_is_the_entrance_reversed() {
        let leaving = Reveal {
            sweep: Sweep {
                at: 0.5,
                easing: Easing::EaseIn,
                backwards: true,
            },
            ..reveal(0.0, 0.0)
        };
        let shown = leaving.shown(0, 1);
        assert_eq!((shown.opacity, shown.drop), (0.75, 2.5));
    }

    #[test]
    fn words_break_at_spaces_and_hold_across_a_held_one() {
        assert_eq!(words("144 partitions"), vec![0..3, 4..14]);
        assert_eq!(words("\u{2007}\u{2007}9 GB"), vec![0..7, 8..10]);
        assert_eq!(cut("  a  ", RevealUnit::Line), vec![0..5]);
        assert_eq!(cut("   ", RevealUnit::Line), Vec::<Range<usize>>::new());
    }
}
