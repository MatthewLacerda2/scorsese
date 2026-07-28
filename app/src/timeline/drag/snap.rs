//! What a drag comes to rest against.
//!
//! Snapping is not a convenience laid on top of dragging, it is what dragging
//! has to feel like. A cut placed by hand lands one frame short about as often
//! as it lands right, and one frame of nothing between two shots renders as a
//! black flash — so the edit is wrong in a way that only shows up on playback.
//!
//! The tolerance arrives in **frames**, converted from pixels by the caller
//! through the one transform ([`super::super::view::View::frames_in`]). That is
//! what makes the pull feel the same magnified and not: on screen it is always
//! the same few pixels, whatever they are worth in time.

use scorsese_core::{ClipId, Frames, Project};

/// Everywhere a drag may come to rest.
///
/// Gathered once per pointer position rather than held across the gesture,
/// because the document changes under a drag — the clip being moved is
/// somewhere new every frame, and a stale list would have it snapping to where
/// it used to be.
pub(in crate::timeline) struct Targets(Vec<Frames>);

/// A snap that was taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::timeline) struct Snap {
    /// The frame landed on — where the indicator line is drawn, so a person
    /// can see *what* they snapped to rather than only that something moved.
    pub(in crate::timeline) at: Frames,
    /// How far the proposal has to shift to reach it. Added to the drag's own
    /// delta and re-proposed, so the shift goes through the same clamps
    /// everything else does.
    pub(in crate::timeline) shift: i64,
}

impl Targets {
    /// The playhead, the start of the timeline, and both edges of every clip
    /// except the one being dragged.
    ///
    /// Every track, not only the one being dragged on: lining a title up with
    /// the cut under it, or a sting with the shot it lands on, is most of what
    /// snapping is for. Order is the order they are listed here, and it decides
    /// ties — see [`Targets::nearest`].
    pub(in crate::timeline) fn gather(
        project: &Project,
        playhead: Frames,
        moving: &ClipId,
    ) -> Self {
        let mut at = vec![playhead, Frames::ZERO];
        for (_, clip) in project.clips().filter(|(_, clip)| &clip.id != moving) {
            at.push(clip.start);
            at.push(clip.end());
        }
        Self(at)
    }

    /// The nearest target within `tolerance` of any of `edges`.
    ///
    /// Nearest wins. A tie goes to whichever target was gathered first, which
    /// puts the playhead ahead of a cut that happens to sit on it — you put the
    /// playhead there deliberately, and the cut only happens to be there.
    /// Between two edges of the moving clip a tie goes to the leading one.
    pub(in crate::timeline) fn nearest(&self, edges: &[Frames], tolerance: Frames) -> Option<Snap> {
        let tolerance = tolerance.get() as i64;
        let mut best: Option<Snap> = None;
        for edge in edges {
            for at in &self.0 {
                let shift = at.get() as i64 - edge.get() as i64;
                if shift.abs() > tolerance {
                    continue;
                }
                if best.is_none_or(|best| shift.abs() < best.shift.abs()) {
                    best = Some(Snap { at: *at, shift });
                }
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_core::{AssetId, Clip, Fps, Track, TrackId, TrackKind};

    fn project(clips: &[(&str, u64, u64)]) -> Project {
        let mut project = Project::new("t", Fps::THIRTY);
        let mut track = Track::new(TrackId::new("v1"), TrackKind::Video);
        for (id, start, duration) in clips {
            track.clips.push(Clip::new(
                ClipId::new(*id),
                AssetId::new("a"),
                Frames(*start),
                Frames(*duration),
            ));
        }
        project.tracks.push(track);
        project
    }

    #[test]
    fn nothing_within_the_tolerance_snaps_to_nothing() {
        let targets = Targets::gather(&project(&[("a", 100, 50)]), Frames(900), &ClipId::new("x"));
        assert_eq!(targets.nearest(&[Frames(500)], Frames(8)), None);
    }

    #[test]
    fn a_clip_edge_within_the_tolerance_pulls_the_drag_onto_it() {
        let targets = Targets::gather(&project(&[("a", 100, 50)]), Frames(900), &ClipId::new("x"));
        let snap = targets
            .nearest(&[Frames(146)], Frames(8))
            .expect("150 is four frames away");
        assert_eq!(snap.at, Frames(150));
        assert_eq!(snap.shift, 4);
    }

    /// Butting a clip up against the one before it is the commonest snap there
    /// is, and it is the *trailing* edge that has to find the target.
    #[test]
    fn either_edge_of_a_move_may_be_the_one_that_finds_a_target() {
        let targets = Targets::gather(&project(&[("a", 100, 50)]), Frames(900), &ClipId::new("x"));
        let snap = targets
            .nearest(&[Frames(20), Frames(98)], Frames(8))
            .expect("the tail is two frames short of 100");
        assert_eq!(snap.at, Frames(100));
        assert_eq!(snap.shift, 2);
    }

    #[test]
    fn the_nearest_target_wins_and_not_the_first_one_found() {
        let targets = Targets::gather(
            &project(&[("a", 100, 50), ("b", 300, 50)]),
            Frames(310),
            &ClipId::new("x"),
        );
        let snap = targets
            .nearest(&[Frames(303)], Frames(20))
            .expect("both 300 and 310 are in reach");
        assert_eq!(snap.at, Frames(300), "three frames beats seven");
    }

    /// You put the playhead where it is; a cut only happens to be there.
    #[test]
    fn a_tie_goes_to_the_playhead() {
        let targets = Targets::gather(&project(&[("a", 100, 50)]), Frames(90), &ClipId::new("x"));
        let snap = targets
            .nearest(&[Frames(95)], Frames(8))
            .expect("90 and 100 are both five frames away");
        assert_eq!(snap.at, Frames(90));
    }

    #[test]
    fn the_start_of_the_timeline_is_a_target_even_with_nothing_on_it() {
        let targets = Targets::gather(&project(&[]), Frames(9_000), &ClipId::new("x"));
        let snap = targets
            .nearest(&[Frames(3)], Frames(8))
            .expect("frame zero");
        assert_eq!(snap.at, Frames::ZERO);
        assert_eq!(snap.shift, -3);
    }

    /// A clip cannot snap to itself: every position would be a snap, and the
    /// drag would never move at all.
    #[test]
    fn the_clip_being_dragged_is_not_one_of_its_own_targets() {
        let targets = Targets::gather(
            &project(&[("a", 100, 50)]),
            Frames(9_000),
            &ClipId::new("a"),
        );
        assert_eq!(
            targets.nearest(&[Frames(101), Frames(151)], Frames(8)),
            None
        );
    }

    /// The same pull in pixels is worth fewer frames the further in the view
    /// is — so a target four frames away is in reach zoomed out and out of
    /// reach zoomed in, which is what makes fine work possible at all.
    #[test]
    fn a_tighter_tolerance_lets_go_of_a_target_a_wider_one_holds() {
        let targets = Targets::gather(
            &project(&[("a", 100, 50)]),
            Frames(9_000),
            &ClipId::new("x"),
        );
        assert!(targets.nearest(&[Frames(104)], Frames(8)).is_some());
        assert_eq!(targets.nearest(&[Frames(104)], Frames(2)), None);
        assert!(
            targets.nearest(&[Frames(100)], Frames::ZERO).is_some(),
            "at the finest zoom only an exact landing counts, and it still counts"
        );
    }
}
