//! What a drag is proposing, in frames — the arithmetic, with no project and
//! no screen anywhere near it.
//!
//! Every proposal is computed from the clip **as it was when the gesture
//! began**, plus the whole pointer travel since. Never from the clip's current
//! state plus one more step: a step the document refused would otherwise be
//! paid for twice, once by not happening and once by the next step starting
//! from somewhere the clip never was.

use scorsese_core::{Clip, Frames};

/// Which part of a clip was taken hold of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::timeline) enum Handle {
    /// The middle: the clip moves, keeping its length and its content.
    Body,
    /// The head. Moves where the clip starts **and** where in the source it
    /// starts, which is the difference between trimming and sliding.
    Left,
    /// The tail. Only the length changes; the head stays put.
    Right,
}

/// How much source a clip has on either side of what it is showing, in frames
/// of the timeline grid.
///
/// The two ceilings a trim comes to rest against, and they are absent for
/// different reasons. `None` for the head means the asset has no timeline of
/// its own — a still, a title, a colour — so pulling its head back is limited
/// by the start of the timeline and nothing else. `None` for the tail means
/// nothing has *measured* a length: the same three, plus a sketch with no file
/// yet, plus a file nobody has probed. A ceiling invented from an absence would
/// stop honest trims on footage we simply have not looked at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::timeline) struct Limits {
    /// How much source lies before the clip's `source_in`.
    pub(in crate::timeline) head: Option<Frames>,
    /// How much source lies past where the clip already ends.
    pub(in crate::timeline) tail: Option<Frames>,
}

/// Where a clip would sit if the pointer were let go now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::timeline) struct Shape {
    /// Where it begins on the timeline.
    pub(in crate::timeline) start: Frames,
    /// How long it runs. Never zero — a clip covering no frame is an edit that
    /// went wrong, and validation says so.
    pub(in crate::timeline) duration: Frames,
    /// Where in the source it begins.
    pub(in crate::timeline) source_in: Frames,
}

impl Shape {
    /// The clip as it stands, untouched — where every proposal starts from.
    pub(in crate::timeline) fn of(clip: &Clip) -> Self {
        Self {
            start: clip.start,
            duration: clip.duration,
            source_in: clip.source_in,
        }
    }

    /// The frame just past the last one it occupies.
    pub(in crate::timeline) fn end(self) -> Frames {
        self.start + self.duration
    }

    /// The edges a snap may take hold of.
    ///
    /// Both of them for a move, because butting a clip up against the one
    /// before it is the same gesture as butting it against the one after —
    /// and only the edge being pulled for a trim, since the other one is not
    /// moving.
    pub(in crate::timeline) fn edges(self, handle: Handle) -> Vec<Frames> {
        match handle {
            Handle::Body => vec![self.start, self.end()],
            Handle::Left => vec![self.start],
            Handle::Right => vec![self.end()],
        }
    }
}

/// The proposal for `handle` dragged `delta` frames from `origin`.
///
/// `limits` is how much source lies either side of what the clip already
/// shows — see [`Limits`] for what an absent one means.
pub(in crate::timeline) fn propose(
    origin: &Clip,
    handle: Handle,
    delta: i64,
    limits: Limits,
) -> Shape {
    let shape = Shape::of(origin);
    let (start, duration, source_in) = (
        shape.start.get() as i64,
        shape.duration.get() as i64,
        shape.source_in.get() as i64,
    );
    match handle {
        // Saturating at frame zero rather than refusing: a drag that runs off
        // the left of the timeline means "put it at the beginning", and the
        // clip keeping its length is the only reading of that which is not a
        // trim nobody asked for.
        Handle::Body => Shape {
            start: Frames(start.saturating_add(delta).max(0) as u64),
            ..shape
        },
        Handle::Left => {
            // Two floors, and they are different limits: the timeline has no
            // frames before zero, and the source has none before its own head.
            let back = match limits.head {
                Some(head) => frames(head).min(start),
                None => start,
            };
            let delta = delta.clamp(-back, duration - 1);
            Shape {
                start: Frames((start + delta) as u64),
                duration: Frames((duration - delta) as u64),
                source_in: Frames((source_in + delta).max(0) as u64),
            }
        }
        // The tail has a ceiling wherever the source has an end: the clip
        // stops where the footage does, the way the head stops at its start.
        // Clamped rather than refused, so the edge rests against the end of
        // the media and the gesture keeps running — the same answer dragging a
        // clip off the front of the timeline gets.
        Handle::Right => {
            let forward = limits.tail.map_or(i64::MAX, frames);
            let delta = delta.min(forward);
            Shape {
                duration: Frames(duration.saturating_add(delta).max(1) as u64),
                ..shape
            }
        }
    }
}

/// A frame count as the signed arithmetic above works in, saturating rather
/// than wrapping on a document that says something absurd.
fn frames(count: Frames) -> i64 {
    i64::try_from(count.get()).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_core::{AssetId, ClipId};

    /// A clip 100 frames long at frame 200, starting 50 frames into its source.
    fn clip() -> Clip {
        let mut clip = Clip::new(
            ClipId::new("c"),
            AssetId::new("a"),
            Frames(200),
            Frames(100),
        );
        clip.source_in = Frames(50);
        clip
    }

    /// An asset with a timeline of its own and no measured length — the
    /// unprobed video that most of these cases are about.
    fn timed(head: u64) -> Limits {
        Limits {
            head: Some(Frames(head)),
            tail: None,
        }
    }

    #[test]
    fn moving_the_body_keeps_the_length_and_the_content() {
        let moved = propose(&clip(), Handle::Body, 30, timed(50));
        assert_eq!(moved.start, Frames(230));
        assert_eq!(moved.duration, Frames(100));
        assert_eq!(moved.source_in, Frames(50), "a move is not a trim");
    }

    #[test]
    fn moving_past_the_start_of_the_timeline_rests_at_the_start() {
        let moved = propose(&clip(), Handle::Body, -9_000, timed(50));
        assert_eq!(moved.start, Frames::ZERO);
        assert_eq!(moved.duration, Frames(100), "clamped, not trimmed");
    }

    /// The whole reason the left edge is its own case: leave `source_in` where
    /// it is and the picture slides sideways under the edge you are dragging,
    /// which is not what anyone means by trimming.
    #[test]
    fn trimming_the_head_moves_where_the_source_starts_too() {
        let trimmed = propose(&clip(), Handle::Left, 20, timed(50));
        assert_eq!(trimmed.start, Frames(220));
        assert_eq!(trimmed.duration, Frames(80));
        assert_eq!(trimmed.source_in, Frames(70));
    }

    #[test]
    fn pulling_the_head_back_gives_the_source_back() {
        let trimmed = propose(&clip(), Handle::Left, -30, timed(50));
        assert_eq!(trimmed.start, Frames(170));
        assert_eq!(trimmed.duration, Frames(130));
        assert_eq!(trimmed.source_in, Frames(20));
    }

    #[test]
    fn the_head_stops_where_the_source_runs_out() {
        let trimmed = propose(&clip(), Handle::Left, -400, timed(50));
        assert_eq!(trimmed.source_in, Frames::ZERO);
        assert_eq!(trimmed.start, Frames(150), "50 frames back, and no further");
        assert_eq!(trimmed.duration, Frames(150));
    }

    /// A title has no source timeline to run out of, so its head is limited by
    /// the start of the timeline and nothing else.
    #[test]
    fn a_still_has_no_head_to_run_out_of() {
        let trimmed = propose(&clip(), Handle::Left, -400, Limits::default());
        assert_eq!(trimmed.start, Frames::ZERO);
        assert_eq!(trimmed.duration, Frames(300));
        assert_eq!(trimmed.source_in, Frames::ZERO);
    }

    #[test]
    fn neither_edge_can_be_dragged_through_the_other() {
        let head = propose(&clip(), Handle::Left, 5_000, timed(50));
        assert_eq!(head.duration, Frames(1));
        assert_eq!(head.start, Frames(299));
        let tail = propose(&clip(), Handle::Right, -5_000, timed(50));
        assert_eq!(tail.duration, Frames(1));
        assert_eq!(tail.start, Frames(200), "the head does not move");
    }

    #[test]
    fn trimming_the_tail_leaves_the_head_and_the_source_alone() {
        let trimmed = propose(&clip(), Handle::Right, 40, timed(50));
        assert_eq!(trimmed.start, Frames(200));
        assert_eq!(trimmed.duration, Frames(140));
        assert_eq!(trimmed.source_in, Frames(50));
    }

    /// The tail's own floor, and the whole point of the issue this answers:
    /// the video goes on for as long as there is content, and no more.
    #[test]
    fn the_tail_stops_where_the_source_runs_out() {
        let limits = Limits {
            head: Some(Frames(50)),
            tail: Some(Frames(30)),
        };
        let trimmed = propose(&clip(), Handle::Right, 400, limits);
        assert_eq!(
            trimmed.duration,
            Frames(130),
            "30 frames on, and no further"
        );
        assert_eq!(trimmed.start, Frames(200), "the head still does not move");
    }

    /// A ceiling only stops the edge going past it. Everything short of it is
    /// the trim the pointer asked for, unchanged.
    #[test]
    fn a_tail_inside_its_source_is_left_where_the_pointer_put_it() {
        let limits = Limits {
            head: Some(Frames(50)),
            tail: Some(Frames(30)),
        };
        assert_eq!(
            propose(&clip(), Handle::Right, 20, limits).duration,
            Frames(120)
        );
    }

    /// A clip already past the end of its source — written before the ceiling
    /// existed, or measured after the fact — can still be pulled back in. Only
    /// the direction that makes it worse is refused.
    #[test]
    fn a_tail_already_past_the_end_may_only_come_back() {
        let limits = Limits {
            head: Some(Frames(50)),
            tail: Some(Frames::ZERO),
        };
        assert_eq!(
            propose(&clip(), Handle::Right, 40, limits).duration,
            Frames(100),
            "held where it is"
        );
        assert_eq!(
            propose(&clip(), Handle::Right, -40, limits).duration,
            Frames(60)
        );
    }

    #[test]
    fn a_move_offers_both_edges_to_a_snap_and_a_trim_only_the_one_pulled() {
        let shape = Shape::of(&clip());
        assert_eq!(shape.edges(Handle::Body), [Frames(200), Frames(300)]);
        assert_eq!(shape.edges(Handle::Left), [Frames(200)]);
        assert_eq!(shape.edges(Handle::Right), [Frames(300)]);
    }
}
