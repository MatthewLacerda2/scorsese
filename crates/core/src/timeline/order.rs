//! The questions every editor asks of one track: where it ends, whether a
//! clip has room on it, and keeping its clips in time order.
//!
//! Each was written by hand at every caller — the room check five times, the
//! sort after a push eight (#1279) — and a mutator that forgets the sort
//! leaves a document validation then reads out of order.

use super::{Clip, Track};
use crate::time::Frames;

impl Track {
    /// The frame just past this track's last clip; zero for an empty track.
    pub fn end(&self) -> Frames {
        self.clips
            .iter()
            .map(Clip::end)
            .max()
            .unwrap_or(Frames::ZERO)
    }

    /// The first clip on this track, in array order, that shares a frame with
    /// `clip` — what a refusal names when there is no room.
    pub fn first_overlap(&self, clip: &Clip) -> Option<&Clip> {
        self.clips.iter().find(|other| other.overlaps(clip))
    }

    /// Whether `clip` fits on this track without overlapping anything on it.
    pub fn is_free_for(&self, clip: &Clip) -> bool {
        self.first_overlap(clip).is_none()
    }

    /// Adds `clip` and keeps the track in time order.
    pub fn insert_clip(&mut self, clip: Clip) {
        self.clips.push(clip);
        self.sort_clips();
    }

    /// Puts the clips back in time order after an edit moved one.
    ///
    /// `project.json` is a document people and agents read, and a track listed
    /// out of order is one nobody can follow. Nothing depends on the order —
    /// clips on a track cannot overlap — so this is for the reader alone. The
    /// sort is stable: clips starting together keep the order they had.
    pub fn sort_clips(&mut self) {
        self.clips.sort_by_key(|clip| clip.start);
    }

    /// The clips in time order, whatever order the document lists them in.
    pub fn in_order(&self) -> Vec<&Clip> {
        let mut ordered: Vec<&Clip> = self.clips.iter().collect();
        ordered.sort_by_key(|clip| clip.start);
        ordered
    }
}
