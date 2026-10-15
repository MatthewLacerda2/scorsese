//! How long the timeline runs, and which of its tracks has room for a clip.

use super::Project;
use crate::time::Frames;
use crate::timeline::{Clip, TrackKind};

impl Project {
    /// The frame just past the last clip on the project's own timeline; zero
    /// for an empty one. A group's members do not count: they play inside
    /// the group's clip, which is on the timeline itself.
    pub fn length(&self) -> Frames {
        self.tracks
            .iter()
            .map(|track| track.end())
            .max()
            .unwrap_or(Frames::ZERO)
    }

    /// The index of the first timeline track of `kind`, at `from` or above it,
    /// with room for `clip` — `None` when none has, and the caller makes a lane.
    ///
    /// "Above" is array order, the order video composites in: a clip that has
    /// to move off its own track goes to the nearest one over it, never under.
    pub fn room_for(&self, kind: TrackKind, from: usize, clip: &Clip) -> Option<usize> {
        self.tracks
            .iter()
            .enumerate()
            .skip(from)
            .find(|(_, track)| track.kind == kind && track.is_free_for(clip))
            .map(|(index, _)| index)
    }
}
