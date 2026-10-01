//! What the follow checks can find: a clip told to travel along something that
//! is not a line it can reach.

use crate::asset::AssetId;
use crate::timeline::ClipId;

/// One thing wrong with a clip's `follow`.
///
/// Its own catalogue rather than a corner of [`super::TimelineProblem`],
/// because every one of these is about a reference from one clip to another —
/// the question a group's edge also asks, and the one an attached arrow asks
/// of its own ends.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FollowProblem {
    /// A `follow` naming a clip the document does not have.
    #[error("clip `{clip}` follows clip `{target}`, which is not in this project")]
    NoSuchClip {
        /// The follower.
        clip: ClipId,
        /// The clip it names.
        target: ClipId,
    },

    /// A `follow` naming a clip that is not an arrow. A box or a title has no
    /// line to travel along, and an arrow is the one path this format draws.
    #[error(
        "clip `{clip}` follows clip `{target}`, which shows `{asset}` — only a clip of an \
         arrow shape has a line to follow"
    )]
    NotAnArrow {
        /// The follower.
        clip: ClipId,
        /// The clip it names.
        target: ClipId,
        /// What that clip shows.
        asset: AssetId,
    },

    /// A clip following itself — which can only happen to an arrow, and would
    /// have it place itself by where it already is.
    #[error("clip `{clip}` follows itself")]
    Itself {
        /// The clip.
        clip: ClipId,
    },

    /// A `follow` across a group's edge: a clip outside a group travelling a
    /// member arrow inside it, or the other way round.
    ///
    /// Refused for [`super::GroupProblem::AttachedAcross`]'s reason: the two
    /// sides are drawn in different spaces, and one group can be on screen
    /// twice at once, so "the arrow" would not name one line.
    #[error(
        "clip `{clip}` follows arrow clip `{target}` across a group's edge — a clip can follow \
         only an arrow on the same timeline as itself: both inside one group, or both outside it"
    )]
    Across {
        /// The follower.
        clip: ClipId,
        /// The arrow clip, on the other side.
        target: ClipId,
    },

    /// A clip following an arrow that is attached to a clip which itself
    /// follows a path.
    ///
    /// **One level deep, on purpose** — the rule that refuses an arrow attached
    /// to an arrow. A path that moves because another path moved is a chain the
    /// renderer would have to order, and a chain that comes back round to its
    /// start (a dot following the arrow attached to that dot) has no order at
    /// all. Attach the arrow to something placed by its own transform instead.
    #[error(
        "clip `{clip}` follows arrow clip `{arrow}`, which is attached to clip `{through}` — and \
         `{through}` follows a path itself; an arrow a clip follows may attach only to clips \
         placed by their own transform"
    )]
    Chained {
        /// The follower.
        clip: ClipId,
        /// The arrow clip it follows.
        arrow: ClipId,
        /// The clip that arrow is attached to, which follows a path.
        through: ClipId,
    },
}
