//! What a press on the timeline means, and what it does while it lasts.
//!
//! Scrubbing and moving a clip are the same gesture as far as a mouse is
//! concerned: a button goes down somewhere and the pointer travels. The
//! difference between them is settled **once, when the drag starts**, from what
//! was under the pointer at that moment — a clip's body or one of its edges
//! starts a move or a trim, the ruler or a bare lane starts a scrub.
//!
//! Deciding it every frame instead would turn a move into a scrub the instant
//! the pointer wandered off the clip, which is exactly what happens on the way
//! to a track two rows down. The gesture is a fact about the hand, not about
//! what is under it now.

use egui::{Pos2, Rect, Response, Ui};
use scorsese_core::{Frames, Project};

use super::view::View;
use super::{Timeline, drag, lanes, pacing, ruler};
use crate::editing::{Editing, length};
use crate::project::Open;

/// What a press on the timeline turned out to mean.
#[derive(Debug)]
pub(super) enum Gesture {
    /// Dragging the playhead.
    Scrub,
    /// Moving or trimming a clip.
    ///
    /// Boxed because a [`drag::Drag`] carries a copy of the clip as it was when
    /// the drag began, and a clip is much bigger than the other variant, which
    /// carries nothing. One press is one allocation, against making every
    /// `Gesture` — including the scrub that is by far the commonest — as large
    /// as a clip.
    Clip(Box<drag::Drag>),
    /// Scaling the selected clips about the playhead.
    ///
    /// The one gesture here that no button started — see [`super::pacing`]. It
    /// belongs in this enum all the same, because what a gesture *is* is the
    /// thing the panel is in the middle of: a reload arriving during one has to
    /// wait for it either way, and this can no more overlap a drag than a drag
    /// can overlap a scrub.
    ///
    /// Boxed for the same reason as the drag, and more so: it carries the whole
    /// document as it stood when the gesture began.
    Pace(Box<pacing::Pace>),
}

impl Timeline {
    /// What a click or drag on the timeline does.
    ///
    /// On the ruler, or on a bare lane: move the playhead. On a clip: select
    /// it, and move or trim it. Scrubbing by dragging anywhere empty is
    /// deliberate — it is the thing you do most, and making it need the thin
    /// strip at the top would make the common case the fiddly one.
    ///
    /// Shift or ctrl held turns a click on a clip into *this one as well*.
    pub(super) fn act(
        &mut self,
        ui: &Ui,
        response: &Response,
        area: Rect,
        hit: Option<&lanes::Hit>,
        open: &mut Open,
        editing: &mut Editing,
    ) {
        if let (None, Some(hit), Some(at)) = (&self.gesture, hit, response.hover_pos()) {
            ui.ctx().set_cursor_icon(drag::cursor(hit, at));
        }
        let Some(at) = response.interact_pointer_pos() else {
            return;
        };
        let on_ruler = at.y <= area.top() + ruler::HEIGHT;

        if response.drag_started() {
            // Where the button went *down*, not where the pointer is now. egui
            // calls it a drag only once the pointer has travelled a few pixels,
            // and reading the later position would both mistake a grab aimed at
            // a clip's edge for one aimed past it and leave the clip trailing
            // the pointer by the threshold for the rest of the gesture.
            let origin = ui.input(|input| input.pointer.press_origin());
            self.grab(origin.unwrap_or(at), area, open, editing, adding(ui));
        }

        if response.dragged() {
            self.follow(ui, area, at, open, editing);
        } else if response.clicked() {
            match hit.filter(|_| !on_ruler) {
                Some(hit) => editing.pick(&hit.clip, adding(ui)),
                None => {
                    // Clicking bare timeline is how you let go of everything —
                    // but the ruler is not bare timeline, it is the playhead's
                    // own strip. Standing the playhead somewhere is half of
                    // every operation that acts on a selection, and a ruler
                    // that cleared the selection would make those two things
                    // impossible to do in either order.
                    if !on_ruler {
                        editing.selected.clear();
                    }
                    editing.playhead = frame_under(self.view, area, at, &open.project);
                }
            }
        }
        if response.drag_stopped() {
            self.write(open);
            self.gesture = None;
        }
    }

    /// Decides what this gesture is, from what the button went down on.
    fn grab(&mut self, at: Pos2, area: Rect, open: &Open, editing: &mut Editing, adding: bool) {
        self.trouble = None;
        let top = area.top() + ruler::HEIGHT;
        let grabbed = (at.y > top)
            .then(|| lanes::hit(&open.project, area, top, self.view, at))
            .flatten();
        self.gesture = Some(match grabbed {
            // Taking hold of a clip selects it: the inspector should be looking
            // at whatever your hand is on. A clip already in the selection is
            // the exception — see [`Editing::hold`].
            Some(hit) => {
                editing.hold(&hit.clip, adding);
                drag::Drag::begin(&open.project, &hit, at, area, self.view)
                    .map_or(Gesture::Scrub, |held| Gesture::Clip(Box::new(held)))
            }
            None => Gesture::Scrub,
        });
    }

    /// One pointer move within a gesture: the clip follows, or the playhead
    /// does.
    fn follow(&mut self, ui: &Ui, area: Rect, at: Pos2, open: &mut Open, editing: &mut Editing) {
        let view = self.view;
        let top = area.top() + ruler::HEIGHT;
        let bypass = ui.input(|input| input.modifiers.alt);
        match &mut self.gesture {
            Some(Gesture::Clip(held)) => {
                let outcome = held.follow(drag::Pull {
                    project: &mut open.project,
                    view,
                    area,
                    top,
                    at,
                    playhead: editing.playhead,
                    bypass,
                });
                self.trouble = outcome.err();
            }
            _ => editing.playhead = frame_under(view, area, at, &open.project),
        }
    }

    /// Puts the edit on disk, once the hand comes off it.
    ///
    /// On release rather than on every pointer move: a drag is dozens of
    /// intermediate positions and none of them is an edit anyone meant to
    /// keep. Scrubbing writes nothing at all — where the playhead sits is a
    /// fact about this session, not about the film.
    fn write(&mut self, open: &mut Open) {
        if matches!(self.gesture, Some(Gesture::Clip(_))) {
            self.trouble = open
                .project
                .save(&open.root)
                .err()
                .map(|why| why.to_string());
        }
    }
}

/// Whether a modifier meaning "as well as" is held.
///
/// Shift **or** ctrl, and both on purpose. Every list anyone has ever clicked
/// in takes one of the two, nobody remembers which belongs to which program,
/// and there is nothing else for either of them to mean on a clip. `command`
/// rather than `ctrl` so the same hand works on a Mac, where it is ⌘.
fn adding(ui: &Ui) -> bool {
    ui.input(|input| input.modifiers.shift || input.modifiers.command)
}

/// Which frame `at` falls on, never past the end of the edit.
fn frame_under(view: View, area: Rect, at: Pos2, project: &Project) -> Frames {
    view.frame_at(at.x - area.left()).min(length(project))
}
