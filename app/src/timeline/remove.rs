//! Removing clips: the Delete key over a selection.
//!
//! `scorsese_core::placing::remove`, the call `clip_remove` makes for an
//! assistant and the web editor — every selected clip at once, validated whole,
//! all or nothing. The assets stay in the project and nothing closes up behind
//! what went: a ripple would move clips nobody selected.

use scorsese_core::{RemoveError, placing};

use super::Timeline;
use super::drag::commit::first_problem;
use crate::editing::Editing;
use crate::project::Open;

impl Timeline {
    /// Takes every selected clip off the timeline and saves, or says why not.
    ///
    /// Nothing selected is nothing to do, and a read-only project is never
    /// written — the same rule every other edit in this panel keeps.
    pub(crate) fn remove_selected(&mut self, open: &mut Open, editing: &mut Editing) {
        if editing.selected.is_empty() || open.read_only() {
            return;
        }
        let mut proposed = open.project.clone();
        self.trouble = match placing::remove(&mut proposed, &editing.selected) {
            // Saved before it is shown, as every other edit here is.
            Ok(_) => match proposed.save(&open.root) {
                Ok(()) => {
                    open.project = proposed;
                    editing.selected.clear();
                    None
                }
                Err(why) => Some(why.to_string()),
            },
            Err(RemoveError::Refused(errors)) => Some(first_problem(errors)),
            Err(other) => Some(other.to_string()),
        };
    }
}
