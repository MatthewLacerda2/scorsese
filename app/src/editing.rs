//! What the window is looking at, as opposed to what the project says.
//!
//! Kept apart from the document on purpose. Where the playhead sits and which
//! clip is selected are facts about *this session*, not about the edit — they
//! are not saved, they do not travel with the project, and nothing renders
//! differently because of them.
//!
//! One home for both, above every panel, because the playhead in particular is
//! **one position with several views**: the scrubber under the preview and the
//! line down the timeline are the same value seen twice. Two copies would
//! disagree the first moment someone dragged one.

use scorsese_core::{ClipId, Frames, Project};

/// Where the window is looking.
#[derive(Debug, Default, Clone)]
pub(crate) struct Editing {
    /// The frame being shown, on the project's grid.
    pub(crate) playhead: Frames,
    /// The selected clip, if any. An id rather than a reference: the document
    /// is mutable and a borrow held across a frame would fight every edit.
    pub(crate) selected: Option<ClipId>,
}

impl Editing {
    /// Forgets everything, for when a different project is opened. A playhead
    /// left at frame 900 of the last film is worse than one at the start.
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    /// Keeps the selection honest after the document changes: a clip that is
    /// no longer there cannot stay selected.
    pub(crate) fn forget_missing(&mut self, project: &Project) {
        if let Some(selected) = &self.selected
            && !project.clips().any(|(_, clip)| &clip.id == selected)
        {
            self.selected = None;
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
