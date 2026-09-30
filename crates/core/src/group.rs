//! Several clips that render as **one layer**: the `group` asset kind.
//!
//! A diagram of thirty boxes, arrows and captions that has to pull back as a
//! whole is thirty clips each carrying the same scale and position keyframes,
//! computed by hand so they stay in formation — and rewritten, all thirty, the
//! first time the move is nudged. A group is the same diagram as one clip with
//! two keyframes. It is Filmora's *compound clip*, and nothing more elaborate
//! than that.
//!
//! **An asset kind, not a `parent` pointer on clips**, and the difference is
//! the reason this exists. A parent pointer gives transform inheritance — move
//! the parent, the children follow — but every child is still its own layer, so
//! a group at half opacity would show its overlapping members through each
//! other, and there would be no single picture to blur, glow or mask. A group
//! asset is a nested composition that renders **once, to one layer**, and a
//! clip of it is then placed, trimmed, animated and composited exactly like any
//! other visual. It also has its own clock, which a pointer does not: the same
//! group can be placed twice, or trimmed, the way a shot can.
//!
//! **Time.** The group's own tracks are laid out from its own zero, and a clip
//! of it is a window onto them exactly as a clip of footage is: the clip's
//! `source_in` is the group frame it opens on, so a group can be trimmed, and
//! placed twice, like a shot. A group clip plays at one group frame per
//! timeline frame; a `speed` on one is refused for now, because a member's
//! keyframes would then fall between frames.
//!
//! **Length is derived, never declared.** How long a group is is where its last
//! member ends ([`Group::length`]). A declared length would be one more number
//! to disagree with the contents it describes, and the disagreement has no good
//! reading either way — a length shorter than the contents hides members
//! nobody deleted, a longer one is empty picture nobody drew. So a group is
//! bounded like footage: a clip of one may not play past its end.
//!
//! **Space.** Members are laid out on the **project's raster**, so every
//! fraction inside a group means what it means outside it, and the group's own
//! transform then applies to the rendered layer as a whole.
//!
//! **Picture only.** A group's tracks are video tracks; sound has no layer to
//! be part of, and mixing members' audio through a group's clock is a feature
//! of its own rather than something to half-do here. Validation says so.

use serde::{Deserialize, Serialize};

use crate::time::Frames;
use crate::timeline::{Clip, Track};

/// What a `group` asset holds: tracks of its own, laid out from its own zero.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    /// The group's lanes, **first at the bottom** — the same shape and the same
    /// order rule as the project's own `tracks`. Every one is a video track.
    ///
    /// Their clips name assets in the project's one assets table, by id, like
    /// any other clip; a group carries placements, never media. A member may be
    /// a clip of another group, which is how a group nests.
    #[serde(default)]
    pub tracks: Vec<Track>,
}

impl Group {
    /// A group holding these tracks.
    pub fn new(tracks: Vec<Track>) -> Self {
        Self { tracks }
    }

    /// How long the group is: the frame just past its last member.
    ///
    /// **Derived rather than declared** — see the module doc. An empty group is
    /// zero frames long, which validation refuses: a layer that can never show
    /// anything is a mistake that would otherwise look like a render failure.
    pub fn length(&self) -> Frames {
        self.clips()
            .map(|(_, clip)| clip.end())
            .max()
            .unwrap_or(Frames::ZERO)
    }

    /// Every member, paired with the track holding it.
    pub fn clips(&self) -> impl Iterator<Item = (&Track, &Clip)> {
        self.tracks
            .iter()
            .flat_map(|track| track.clips.iter().map(move |clip| (track, clip)))
    }
}
