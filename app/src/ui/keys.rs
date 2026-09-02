//! The keyboard: navigating the film without aiming at anything.
//!
//! None of these is invented. Space plays, the arrows step a frame, `Home` and
//! `End` are the ends of the edit, `F` fits the timeline to it — that is the
//! keyboard of every editor anybody has used, and a preview is not the place to
//! be interesting.
//!
//! It is here rather than in the panels because a key is not a fact about a
//! panel. The playhead belongs to [`Editing`](crate::editing::Editing), above
//! every panel, precisely because the scrubber under the preview and the line
//! down the timeline are one position with two views — and a key that moved it
//! would be a third owner if it lived in either.
//!
//! ## The guard
//!
//! Every key here is read only when nothing has the keyboard
//! (`egui_wants_keyboard_input`) and no gesture is in flight. The inspector has
//! text fields in it, and a window whose spacebar plays the film while somebody
//! is typing a prompt into one is a window nobody can use. The pacing gesture
//! already made this argument for `S`; this inherits it rather than restating
//! it, which is why the check is in one place at the top.
//!
//! `Space` is the one key here with no test of its own: starting the transport
//! spawns the mixer and then advances the playhead off a clock, so anything
//! asserted after it is a race against a thread. It and the button under the
//! preview call one method between them, which is what the rest of the suite
//! covers.

use egui::{Key, Ui};
use scorsese_core::Frames;

use super::Scorsese;
use crate::timeline::Look;

impl Scorsese {
    /// Reads the keyboard and moves the playhead, the transport or the view.
    pub(super) fn follow_keyboard(&mut self, ui: &Ui) {
        if ui.ctx().egui_wants_keyboard_input() || self.timeline.busy() {
            return;
        }
        // Read out of the document and let go of it again. What follows takes
        // `&mut self` in half its arms, and a borrow of the project held across
        // the loop would make every one of them a fight with the compiler for
        // no reason — these are two numbers.
        let Some((fps, last)) = self.opened.as_ref().map(|open| {
            (
                open.project.timeline_fps,
                crate::preview::last_frame(&open.project),
            )
        }) else {
            return;
        };
        // Read off the **events**, each with the modifiers that were held when
        // it was sent, rather than off `input.modifiers`. That field is the
        // state at the end of the frame's input, which is the state after the
        // key came back up — so a shift-arrow arriving in one batch reads as a
        // plain arrow, and the step that is supposed to be a second is a frame.
        // Taking the modifiers from the press itself also gets auto-repeat
        // right, which is what a held arrow is.
        let pressed: Vec<(Key, bool)> = ui.input(|input| {
            input
                .events
                .iter()
                .filter_map(|event| match event {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => Some((*key, modifiers.shift)),
                    _ => None,
                })
                .collect()
        });

        for (key, shift) in pressed {
            // Re-read inside the loop, not once above it. Auto-repeat can put
            // two presses in one frame, and both stepping from the position the
            // frame started at would make the second one do nothing.
            let at = self.editing.playhead;
            // A second is what shift means, at whatever rate the project is
            // authored at — a jump of a fixed number of frames would be a
            // different distance in every project.
            let stride = if shift {
                fps.frames(1.0).get().max(1)
            } else {
                1
            };
            match key {
                // Pressing play while parked at the end starts over — the
                // transport's own rule, and this is a second way to press the
                // same button rather than a second transport.
                Key::Space => {
                    if let Some(open) = self.opened.as_ref() {
                        self.preview.toggle_playback(open, at, last);
                    }
                }
                Key::ArrowLeft => self.seek(Frames(at.get().saturating_sub(stride)), last),
                Key::ArrowRight => self.seek(Frames(at.get().saturating_add(stride)), last),
                Key::Home => self.seek(Frames::ZERO, last),
                Key::End => self.seek(last, last),
                Key::F => self.timeline.ask(Look::Fit),
                // `+` is shift and `=` on most keyboards, and which of the two
                // a platform reports is not something to make a person find out.
                Key::Plus | Key::Equals => self.timeline.ask(Look::In),
                Key::Minus => self.timeline.ask(Look::Out),
                _ => {}
            }
        }
    }

    /// Puts the playhead somewhere and stops playback.
    ///
    /// Stopping is what every editor does and is the whole reason this is not
    /// two lines at each call site: a step or a jump is somebody taking hold of
    /// the playhead, and having it run away again is never what was meant.
    fn seek(&mut self, to: Frames, last: Frames) {
        self.editing.playhead = to.min(last);
        self.preview.stop();
    }
}
