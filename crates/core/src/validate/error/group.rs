//! What the group checks can find: a group that could never be drawn, and a
//! reference that crosses a group's edge.

use crate::asset::AssetId;
use crate::timeline::{ClipId, TrackId};

/// One thing wrong with a `group` asset, a clip of one, or a reference that
/// crosses a group's boundary.
///
/// Its own catalogue rather than a corner of [`super::AssetProblem`] or
/// [`super::TimelineProblem`], because a group is both at once: an asset whose
/// content is tracks, and so something the timeline checks and the asset checks
/// each see half of.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum GroupProblem {
    /// A group with no clip in it: zero frames long, and a layer that could
    /// never show anything — which looks exactly like a render that failed.
    #[error("group `{asset}` has no clips in it, so there is nothing for it to draw")]
    Empty {
        /// The empty group.
        asset: AssetId,
    },

    /// An audio track inside a group. A group is **one picture**: sound has no
    /// layer to be part of, and mixing members' audio through a group's own
    /// clock is not something this build does.
    #[error(
        "group `{asset}` holds audio track `{track}` — a group is picture only; put sound on \
         the project's own audio tracks"
    )]
    SoundInGroup {
        /// The group.
        asset: AssetId,
        /// The audio track inside it.
        track: TrackId,
    },

    /// A group that contains a clip of itself, directly or through other
    /// groups. It would take drawing itself to draw itself.
    #[error("group `{asset}` contains itself, directly or through another group")]
    ContainsItself {
        /// A group on the cycle.
        asset: AssetId,
    },

    /// A clip of a group with a `speed` other than `1`.
    ///
    /// Refused rather than approximated: at any other rate a group frame falls
    /// between two timeline frames, and so does every member's keyframes. A
    /// group plays at its own pace — retime its members instead.
    #[error(
        "clip `{clip}` plays group `{asset}` at speed {speed}; a group clip plays at 1 — \
         retime the members inside it instead"
    )]
    AtSpeed {
        /// The clip.
        clip: ClipId,
        /// The group it shows.
        asset: AssetId,
        /// The rate as written.
        speed: f64,
    },

    /// An arrow attached to a clip on the other side of a group's edge: an
    /// arrow outside a group following a member inside it, or the other way
    /// round.
    ///
    /// **Refused for now, and on purpose.** Inside a group an arrow resolves in
    /// the group's own space, before the group's transform; outside, on the
    /// frame, after it. An arrow crossing the boundary would have to be drawn
    /// in one space and aimed at a point in the other — and the same group can
    /// be placed twice at once, so "the member" would not even name one place.
    /// Attach the arrow to the clip of the group instead, or put the arrow in
    /// the group beside what it follows.
    #[error(
        "clip `{clip}` draws arrow `{arrow}`, attached to clip `{target}` across a group's edge \
         — an arrow can follow only clips on the same timeline as itself: both inside one \
         group, or both outside it"
    )]
    AttachedAcross {
        /// The arrow's clip.
        clip: ClipId,
        /// The arrow asset.
        arrow: AssetId,
        /// The clip it follows, on the other side.
        target: ClipId,
    },
}
