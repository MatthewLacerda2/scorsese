//! What the window is looking at, as opposed to what the project says.
//!
//! Kept apart from the document on purpose. Where the playhead sits and which
//! clips are selected are facts about *this session*, not about the edit — they
//! are not saved, they do not travel with the project, and nothing renders
//! differently because of them.
//!
//! One home for both, above every panel, because the playhead in particular is
//! **one position with several views**: the scrubber under the preview and the
//! line down the timeline are the same value seen twice. Two copies would
//! disagree the first moment someone dragged one.

use std::collections::BTreeSet;

use scorsese_core::{AssetId, ClipId, Frames, Project};

/// Where the window is looking.
#[derive(Debug, Default, Clone)]
pub(crate) struct Editing {
    /// The frame being shown, on the project's grid.
    pub(crate) playhead: Frames,
    /// Every selected clip, and often none. Ids rather than references: the
    /// document is mutable and a borrow held across a frame would fight every
    /// edit.
    ///
    /// A **set**, and an ordered one. A set because a clip is selected or it is
    /// not, and a list that could hold the same clip twice would make every
    /// operation over the selection ask whether it had already done that one.
    /// Ordered because the moment something iterates the selection — scaling a
    /// run of clips, deleting them, nudging them — the order it visits them in
    /// decides which refusal a person meets first, and that must not depend on
    /// the order they happened to be clicked in.
    pub(crate) selected: BTreeSet<ClipId>,
    /// An asset picked out in the files panel, whose clips the timeline picks
    /// out in turn. Answers "where does this actually get used?", which a list
    /// of assets cannot.
    pub(crate) highlighted: Option<AssetId>,
}

impl Editing {
    /// Forgets everything, for when a different project is opened. A playhead
    /// left at frame 900 of the last film is worse than one at the start.
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    /// Keeps the selection honest after the document changes: a clip that is
    /// no longer there cannot stay selected.
    ///
    /// Prunes rather than clears. An outside edit that removed one clip has
    /// said nothing whatever about the other four, and dropping them too would
    /// make somebody else's save undo a selection this window was mid-gesture
    /// on.
    pub(crate) fn forget_missing(&mut self, project: &Project) {
        if self.selected.is_empty() {
            return;
        }
        let alive: BTreeSet<&ClipId> = project.clips().map(|(_, clip)| &clip.id).collect();
        self.selected.retain(|clip| alive.contains(clip));
    }

    /// Picks a clip, as clicking it does.
    ///
    /// `adding` is the modifier held: without it a click *replaces* the
    /// selection, with it the clip joins or leaves one that is already there.
    /// Toggling rather than only adding is what makes a modifier-click undoable
    /// by itself — the alternative leaves a mis-clicked clip in the selection
    /// with no way out but starting over.
    pub(crate) fn pick(&mut self, clip: &ClipId, adding: bool) {
        if !adding {
            self.selected.clear();
        } else if self.selected.remove(clip) {
            return;
        }
        self.selected.insert(clip.clone());
    }

    /// Takes hold of a clip, as putting a hand on it to drag does.
    ///
    /// The same as [`Editing::pick`], except that a clip **already selected
    /// leaves the selection alone**. Grabbing one of four selected clips is how
    /// a person starts moving them, and collapsing the four to one on the way
    /// down would throw the selection away at exactly the moment it was about
    /// to be used.
    pub(crate) fn hold(&mut self, clip: &ClipId, adding: bool) {
        if !self.selected.contains(clip) {
            self.pick(clip, adding);
        }
    }

    /// The selected clip when exactly one is — the subject an inspector that
    /// edits one value at a time needs, and `None` the moment there is no
    /// single answer to "which clip is this about?".
    pub(crate) fn only(&self) -> Option<&ClipId> {
        match self.selected.len() {
            1 => self.selected.first(),
            _ => None,
        }
    }
}

/// The frame just past the last one anything occupies — the length of the
/// edit, which is what a view has to fit and a playhead has to stop at.
pub(crate) fn length(project: &Project) -> Frames {
    project
        .clips()
        .map(|(_, clip)| clip.end())
        .max()
        .unwrap_or(Frames::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_core::{AssetId, Clip, Fps, Track, TrackId, TrackKind};

    fn clip(id: &str) -> ClipId {
        ClipId::new(id)
    }

    /// Two clips on a picture track and one on a sound track. Enough for the
    /// only document question asked here: is this clip still in it?
    fn project() -> Project {
        let mut project = Project::new("test", Fps::THIRTY);
        let mut video = Track::new(TrackId::new("v1"), TrackKind::Video);
        video.clips = vec![laid("c1", 0, 90), laid("c2", 120, 60)];
        let mut audio = Track::new(TrackId::new("a1"), TrackKind::Audio);
        audio.clips = vec![laid("c3", 0, 180)];
        project.tracks = vec![video, audio];
        project
    }

    fn laid(id: &str, start: u64, duration: u64) -> Clip {
        Clip::new(
            clip(id),
            AssetId::new("shot"),
            Frames(start),
            Frames(duration),
        )
    }

    fn picked(editing: &Editing) -> Vec<&str> {
        editing.selected.iter().map(ClipId::as_str).collect()
    }

    /// A plain click is the commonest thing anyone does here, and it means
    /// *this one instead of those* — never *this one as well*.
    #[test]
    fn a_click_replaces_the_selection_and_a_modifier_click_adds_to_it() {
        let mut editing = Editing::default();
        editing.pick(&clip("c1"), false);
        editing.pick(&clip("c2"), false);
        assert_eq!(picked(&editing), ["c2"], "a plain click replaces");

        editing.pick(&clip("c1"), true);
        assert_eq!(picked(&editing), ["c1", "c2"], "a modifier click adds");
    }

    /// The way back out of a mis-click, and the reason adding is a toggle.
    #[test]
    fn a_modifier_click_on_a_selected_clip_takes_it_back_out() {
        let mut editing = Editing::default();
        editing.pick(&clip("c1"), false);
        editing.pick(&clip("c2"), true);
        editing.pick(&clip("c1"), true);
        assert_eq!(picked(&editing), ["c2"]);
    }

    /// Whatever order they were clicked in, the selection reads the same —
    /// which is what anything iterating it depends on.
    #[test]
    fn the_selection_is_ordered_rather_than_kept_in_click_order() {
        let mut editing = Editing::default();
        for id in ["c3", "c1", "c2"] {
            editing.pick(&clip(id), true);
        }
        assert_eq!(picked(&editing), ["c1", "c2", "c3"]);
    }

    /// Putting a hand on a clip that is already part of a group must not throw
    /// the group away — that is the moment the group is about to be used.
    #[test]
    fn grabbing_a_selected_clip_keeps_the_whole_selection() {
        let mut editing = Editing::default();
        editing.pick(&clip("c1"), false);
        editing.pick(&clip("c2"), true);

        editing.hold(&clip("c2"), false);
        assert_eq!(picked(&editing), ["c1", "c2"]);

        editing.hold(&clip("c3"), false);
        assert_eq!(picked(&editing), ["c3"], "a clip outside it starts afresh");
    }

    /// A document change that removes one clip has said nothing about the
    /// others, so it must not drop them.
    #[test]
    fn a_clip_that_has_gone_is_pruned_and_the_rest_stay() {
        let project = project();
        let mut editing = Editing::default();
        for id in ["c1", "c3", "gone"] {
            editing.pick(&clip(id), true);
        }
        editing.forget_missing(&project);
        assert_eq!(picked(&editing), ["c1", "c3"]);
    }

    /// One subject or none: the inspector's editing controls are about a clip,
    /// and three clips are not a clip.
    #[test]
    fn there_is_a_single_subject_only_when_exactly_one_is_selected() {
        let mut editing = Editing::default();
        assert!(editing.only().is_none());
        editing.pick(&clip("c1"), false);
        assert_eq!(editing.only(), Some(&clip("c1")));
        editing.pick(&clip("c2"), true);
        assert!(editing.only().is_none());
    }
}
