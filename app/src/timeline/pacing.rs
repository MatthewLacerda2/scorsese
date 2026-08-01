//! Scaling the selected clips about the playhead — the pacing gesture.
//!
//! **Modal, and the only gesture here that is.** Every other thing this panel
//! does begins with a button going down on something: the pointer is the noun
//! and the press is the verb. This one has no noun to press on — the clips are
//! already chosen and the instant is already stood on — so `S` arms it, the
//! pointer then drives a number, and a click or `Enter` keeps it.
//!
//! A drag with continuous feedback rather than a field, because the value you
//! want is the one that *looks* right. Nobody knows in advance that a montage
//! wants ×0.92; they know it when the clips are in the right places.
//!
//! Like [`super::drag`], nothing accumulates: every pointer position proposes
//! an absolute factor against the document **as it was when the gesture was
//! armed**. A factor the document refuses therefore costs the gesture nothing,
//! and the clips simply stop moving — which is what the issue's "clamped rather
//! than allowed-then-rejected" feels like from the hand's side.

use std::collections::BTreeSet;

use egui::{Key, Ui};
use scorsese_core::{ClipId, Frames, Project, pacing};

use super::{Gesture, Timeline};
use crate::editing::Editing;
use crate::project::Open;

/// How far the pointer travels, in pixels, to double or halve the spacing.
const DOUBLING: f32 = 240.0;

/// How far the gesture may be pushed either way.
///
/// Wide enough for any retime anyone means and bounded so a pointer flung to
/// the edge of a monitor does not ask for a factor of four hundred, whose only
/// possible answer is a refusal.
const RANGE: (f64, f64) = (0.05, 20.0);

/// A scale in flight.
#[derive(Debug)]
pub(super) struct Pace {
    /// The document as it was when the gesture was armed. Every proposal is
    /// measured from here, so letting go without moving leaves the project
    /// untouched and `Esc` has something exact to go back to.
    origin: Project,
    /// Where the edit is pinned: the playhead when the gesture began.
    about: Frames,
    /// Where the pointer was then.
    from: f32,
    /// The last factor the document accepted.
    factor: f64,
    /// How many clips that factor moved without stretching, because the media
    /// behind them has a length of its own.
    kept: usize,
}

impl Pace {
    /// One pointer position: proposes a factor and writes the result into the
    /// document, or says why it could not, having changed nothing.
    fn follow(
        &mut self,
        x: f32,
        project: &mut Project,
        clips: &BTreeSet<ClipId>,
    ) -> Option<String> {
        let factor = factor_for(x - self.from);
        let mut proposed = self.origin.clone();
        match pacing::scale(&mut proposed, clips, self.about, factor) {
            Ok(paced) => {
                self.factor = factor;
                self.kept = paced.kept.len();
                *project = proposed;
                None
            }
            Err(why) => Some(why.to_string()),
        }
    }

    /// Puts the document back exactly as it was, for `Esc`.
    fn undo(&self, project: &mut Project) {
        project.clone_from(&self.origin);
    }

    /// What the timeline says while this is running.
    ///
    /// The factor, because a gesture driven by pointer travel has no other way
    /// to tell you where it has got to; what it declined to stretch, because
    /// that is the one thing it does differently from what was asked; and the
    /// two keys, because this is the first gesture in the window that a mouse
    /// alone cannot find.
    pub(super) fn readout(&self) -> String {
        let kept = match self.kept {
            0 => String::new(),
            1 => " · 1 clip keeps the length its media has".to_owned(),
            many => format!(" · {many} clips keep the length their media has"),
        };
        format!(
            "scaling ×{:.2}{kept} — click or Enter to keep, Esc to cancel",
            self.factor
        )
    }
}

impl Timeline {
    /// The pacing gesture, from the key that arms it to the click that keeps
    /// it. `true` while one is live, which is how [`Timeline::show`] knows not
    /// to also read that click as a click on a clip.
    pub(super) fn pace(&mut self, ui: &Ui, open: &mut Open, editing: &Editing) -> bool {
        if !matches!(self.gesture, Some(Gesture::Pace(_))) {
            self.arm(ui, open, editing);
            return matches!(self.gesture, Some(Gesture::Pace(_)));
        }
        let hand = Hand::read(ui);
        let Some(Gesture::Pace(pace)) = &mut self.gesture else {
            return false;
        };
        if let Some(x) = hand.x {
            self.trouble = pace.follow(x, &mut open.project, &editing.selected);
        }
        if hand.cancel {
            pace.undo(&mut open.project);
            self.gesture = None;
            self.trouble = None;
        } else if hand.commit {
            self.gesture = None;
            // Saved here and not on every pointer move, for the same reason a
            // drag is: the dozens of factors on the way to this one are not
            // edits anybody meant to keep.
            self.trouble = open
                .project
                .save(&open.root)
                .err()
                .map(|why| why.to_string());
        }
        true
    }

    /// Arms the gesture, if `S` was pressed and there is anything to scale.
    ///
    /// The keyboard is not modal in this window, so the check that a text field
    /// has focus is what stops an inspector being typed into from arming this.
    fn arm(&mut self, ui: &Ui, open: &Open, editing: &Editing) {
        if !ui.input(|input| input.key_pressed(Key::S)) || ui.ctx().egui_wants_keyboard_input() {
            return;
        }
        if editing.selected.is_empty() {
            // Said rather than ignored. A key that does nothing looks like a
            // key that does not exist, and this one is hard enough to find.
            self.trouble = Some("select the clips to scale first".to_owned());
            return;
        }
        let Some(at) = ui.ctx().pointer_latest_pos() else {
            return;
        };
        self.trouble = None;
        self.gesture = Some(Gesture::Pace(Box::new(Pace {
            origin: open.project.clone(),
            about: editing.playhead,
            from: at.x,
            factor: 1.0,
            kept: 0,
        })));
    }
}

/// What the hand is doing this frame.
///
/// Read from the context rather than from the panel's own response, because
/// once this is armed the pointer is driving a number and may be anywhere at
/// all — including outside the timeline, which is where it ends up the moment
/// anyone asks for a factor bigger than the panel is wide.
struct Hand {
    /// Where the pointer is, when it is anywhere.
    x: Option<f32>,
    /// Keep what is on screen.
    commit: bool,
    /// Put it all back.
    cancel: bool,
}

impl Hand {
    fn read(ui: &Ui) -> Self {
        ui.input(|input| Self {
            x: input.pointer.latest_pos().map(|at| at.x),
            commit: input.pointer.any_click() || input.key_pressed(Key::Enter),
            cancel: input.key_pressed(Key::Escape),
        })
    }
}

/// The factor a pointer travel of `pixels` asks for.
///
/// Exponential, so equal travel is equal *proportional* change in both
/// directions: 240 pixels right doubles the spacing and 240 left halves it. A
/// linear ramp cannot be symmetric — the travel that stretches a cut to twice
/// its length would have to shrink it past zero to match — so one direction
/// would always be the cramped one, and tightening is the direction people
/// reach for most.
fn factor_for(pixels: f32) -> f64 {
    let asked = f64::from(2.0_f32.powf(pixels / DOUBLING));
    asked.clamp(RANGE.0, RANGE.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standing_still_asks_for_no_change_at_all() {
        assert!((factor_for(0.0) - 1.0).abs() < 1e-9);
    }

    /// The reason it is exponential: the same travel means the same amount of
    /// change whichever way it goes.
    #[test]
    fn equal_travel_either_way_doubles_and_halves() {
        assert!((factor_for(DOUBLING) - 2.0).abs() < 1e-6);
        assert!((factor_for(-DOUBLING) - 0.5).abs() < 1e-6);
        assert!((factor_for(2.0 * DOUBLING) - 4.0).abs() < 1e-6);
    }

    /// A pointer flung at a monitor's edge asks for a factor whose only
    /// possible answer is a refusal, and a gesture that only ever refuses is
    /// one nobody can tell is working.
    #[test]
    fn the_far_ends_of_the_travel_are_bounded() {
        assert!((factor_for(100_000.0) - RANGE.1).abs() < 1e-9);
        assert!((factor_for(-100_000.0) - RANGE.0).abs() < 1e-9);
    }
}
