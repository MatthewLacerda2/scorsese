//! Which parts of a clip something else is playing over.
//!
//! Pure interval arithmetic on the timeline grid — no audio, no assets, and
//! nothing that knows why anyone wants to know.

use crate::time::Frames;
use crate::timeline::Clip;

/// A stretch of one clip, in frames **relative to that clip's start**.
///
/// Clip-relative rather than absolute because that is what a keyframe time is,
/// and converting once here is better than converting at every use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Span {
    /// First frame covered.
    pub(crate) from: Frames,
    /// The frame just past the last one covered.
    pub(crate) to: Frames,
}

impl Span {
    pub(crate) fn is_empty(self) -> bool {
        self.from >= self.to
    }
}

/// The spans of `clip` that `others` cover, clip-relative and in order.
///
/// A clip that does not overlap contributes nothing; one that overlaps
/// partially contributes only the shared part. Clips that merely touch — one
/// ending exactly where the other starts — do not overlap, the same rule
/// [`Clip::overlaps`] holds everywhere else.
pub(crate) fn covering<'a>(clip: &Clip, others: impl IntoIterator<Item = &'a Clip>) -> Vec<Span> {
    let mut spans: Vec<Span> = others
        .into_iter()
        .filter(|other| other.overlaps(clip))
        .map(|other| Span {
            from: Frames(other.start.get().saturating_sub(clip.start.get())),
            to: Frames(other.end().get().saturating_sub(clip.start.get())).min(clip.duration),
        })
        .filter(|span| !span.is_empty())
        .collect();
    spans.sort_unstable();
    spans
}

/// Merges spans that overlap, and spans separated by less than `gap`.
///
/// The second half is the one that matters. Two lines of narration a moment
/// apart are one dip, not two: coming all the way back up and immediately
/// going down again is pumping, which draws more attention than the ducking
/// was avoiding. `gap` is how much silence is worth surfacing for.
pub(crate) fn merge(spans: &[Span], gap: Frames) -> Vec<Span> {
    let mut sorted: Vec<Span> = spans.iter().copied().filter(|s| !s.is_empty()).collect();
    sorted.sort_unstable();

    let mut merged: Vec<Span> = Vec::with_capacity(sorted.len());
    for span in sorted {
        match merged.last_mut() {
            Some(last) if span.from <= last.to + gap => last.to = last.to.max(span.to),
            _ => merged.push(span),
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(from: u64, to: u64) -> Span {
        Span {
            from: Frames(from),
            to: Frames(to),
        }
    }

    #[test]
    fn separate_spans_stay_separate() {
        let merged = merge(&[span(0, 10), span(100, 110)], Frames(5));
        assert_eq!(merged, vec![span(0, 10), span(100, 110)]);
    }

    #[test]
    fn overlapping_spans_become_one() {
        let merged = merge(&[span(0, 60), span(30, 90)], Frames::ZERO);
        assert_eq!(merged, vec![span(0, 90)]);
    }

    /// The rule that stops the level pumping between two nearby lines.
    #[test]
    fn spans_closer_than_the_gap_become_one() {
        let merged = merge(&[span(0, 30), span(40, 70)], Frames(27));
        assert_eq!(merged, vec![span(0, 70)], "a 10-frame gap is not worth it");

        let merged = merge(&[span(0, 30), span(40, 70)], Frames(5));
        assert_eq!(merged.len(), 2, "with a short gap they stay apart");
    }

    #[test]
    fn merging_does_not_depend_on_the_order_given() {
        let forward = merge(&[span(0, 30), span(20, 50), span(80, 90)], Frames::ZERO);
        let backward = merge(&[span(80, 90), span(20, 50), span(0, 30)], Frames::ZERO);
        assert_eq!(forward, backward);
    }

    #[test]
    fn an_empty_span_is_dropped_rather_than_merged() {
        assert_eq!(merge(&[span(10, 10)], Frames::ZERO), vec![]);
        assert!(span(10, 10).is_empty());
        assert!(span(10, 5).is_empty(), "backwards is empty too");
    }
}
