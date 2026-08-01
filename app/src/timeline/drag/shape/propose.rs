//! Turning a pointer's travel into the clip it is proposing.
//!
//! Every proposal is absolute — computed from the clip as it was when the
//! gesture began plus the whole travel since — so a step the document refused
//! costs the gesture nothing and the next step does not start from somewhere
//! the clip never was.

use scorsese_core::{Clip, Frames};

use super::{Handle, Limits, Shape};

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
            //
            // A frame of timeline is also not a frame of source once the clip
            // has a speed: at 2× dragging the head one frame in eats two frames
            // of the media. So the source-side floor is converted into the
            // *timeline* it is worth before it can bound a pointer's travel.
            let speed = origin.speed;
            let back = match limits.head {
                Some(head) => (speed.timeline_frames(head.get() as f64) as i64).min(start),
                None => start,
            };
            let delta = delta.clamp(-back, duration - 1);
            let consumed = speed.source_frames(Frames(delta.unsigned_abs())).round() as i64;
            let consumed = if delta < 0 { -consumed } else { consumed };
            Shape {
                start: Frames((start + delta) as u64),
                duration: Frames((duration - delta) as u64),
                source_in: Frames((source_in + consumed).max(0) as u64),
            }
        }
        // The tail has a ceiling wherever the source has an end: the clip
        // stops where the footage does, the way the head stops at its start.
        // Clamped rather than refused, so the edge rests against the end of
        // the media and the gesture keeps running — the same answer dragging a
        // clip off the front of the timeline gets.
        Handle::Right => {
            // Source-side, like the head's floor, so converted the same way:
            // at 2× the thirty frames left in the media are worth fifteen
            // frames of timeline, and clamping at thirty would trim past the
            // end this exists to stop at.
            let forward = limits.tail.map_or(i64::MAX, |tail| {
                origin.speed.timeline_frames(tail.get() as f64) as i64
            });
            let delta = delta.min(forward);
            Shape {
                duration: Frames(duration.saturating_add(delta).max(1) as u64),
                ..shape
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_core::{AssetId, ClipId, Speed};

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

    /// A frame of timeline stops being a frame of source the moment a clip has
    /// a speed. Pulling the head in 20 frames at 2× consumes 40 of the media,
    /// and a trim that moved `source_in` by 20 would slide the picture out from
    /// under the edge being dragged — the exact failure the left handle exists
    /// to avoid.
    #[test]
    fn trimming_the_head_of_a_sped_clip_eats_source_at_its_own_rate() {
        let mut clip = clip();
        clip.speed = Speed::new(2.0);
        let trimmed = propose(
            &clip,
            Handle::Left,
            20,
            Limits {
                head: Some(Frames(50)),
                tail: None,
            },
        );
        assert_eq!(trimmed.start, Frames(220));
        assert_eq!(trimmed.duration, Frames(80));
        assert_eq!(trimmed.source_in, Frames(90), "50 + 20 timeline frames × 2");
    }

    /// And the floor is in the same currency. Fifty frames of material before
    /// the head is only twenty-five frames of *timeline* at 2×, so that is how
    /// far back the edge may be pulled.
    #[test]
    fn the_head_of_a_sped_clip_runs_out_of_source_sooner() {
        let mut clip = clip();
        clip.speed = Speed::new(2.0);
        let trimmed = propose(
            &clip,
            Handle::Left,
            -400,
            Limits {
                head: Some(Frames(50)),
                tail: None,
            },
        );
        assert_eq!(trimmed.source_in, Frames::ZERO);
        assert_eq!(trimmed.start, Frames(175), "25 frames back, and no further");
        assert_eq!(trimmed.duration, Frames(125));
    }

    #[test]
    fn a_move_offers_both_edges_to_a_snap_and_a_trim_only_the_one_pulled() {
        let shape = Shape::of(&clip());
        assert_eq!(shape.edges(Handle::Body), [Frames(200), Frames(300)]);
        assert_eq!(shape.edges(Handle::Left), [Frames(200)]);
        assert_eq!(shape.edges(Handle::Right), [Frames(300)]);
    }
}
