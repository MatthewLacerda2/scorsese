//! Removing clips: the Delete key over a selection.
//!
//! `scorsese_core::placing::remove`, the call `clip_remove` makes for an
//! assistant and the web editor — every selected clip at once, validated whole,
//! all or nothing. The assets stay in the project and nothing closes up behind
//! what went: a ripple would move clips nobody selected.
//!
//! A lane's head right-clicked asks to remove the whole track instead, which is
//! a confirm listing the clips on it ([`crate::removing`]) rather than a key.

use egui::{Pos2, Rect, Response};
use scorsese_core::{RemoveError, placing};

use super::Timeline;
use super::drag::commit::first_problem;
use crate::editing::Editing;
use crate::project::Open;
use crate::removing::{Asking, Removal};

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

    /// Asks to remove the track whose head was right-clicked, if one was.
    pub(super) fn ask_on_head(
        response: &Response,
        pointer: Option<Pos2>,
        gutter: Rect,
        open: &Open,
        editing: &mut Editing,
    ) {
        if !response.secondary_clicked() {
            return;
        }
        let top = gutter.top() + super::ruler::HEIGHT;
        let head = pointer
            .filter(|at| gutter.contains(*at))
            .and_then(|at| super::lanes::lane_at(&open.project, gutter, top, at.y));
        if let Some((track, _)) = head {
            editing.asking = Some(Asking::about(Removal::Track(track.id.clone())));
        }
    }
}
