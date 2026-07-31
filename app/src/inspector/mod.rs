//! The inspector: what is selected, and the plain things about it a person
//! changes with a mouse.
//!
//! It answers the question a timeline raises and cannot itself answer — *what
//! am I looking at?* — and then lets the small direct changes happen where the
//! answer is, because they are faster to make than to describe.
//!
//! Three rules shape it, and all three are about not lying.
//!
//! **Every change goes through the document.** It is applied to a copy,
//! validated, written to disk, and only then shown — so a value on screen is a
//! value `scorsese render` would use, and a change the project could not
//! survive is refused with the reason rather than saved and regretted. See
//! [`edit`].
//!
//! **Nothing here edits a keyframe.** A property something animates is shown as
//! animated and offered no single value: one field cannot hold a ramp, and
//! writing one over an animation would flatten work nobody asked to lose.
//! Animating a property over time is structured work, and structured work is a
//! sentence to an assistant rather than a drag.
//!
//! **A field needs a subject.** Every control here is about *this clip*, so
//! there are three states and not two: nothing selected, one clip, and several.
//! [`one`] draws the panel a field belongs in; [`several`] draws what can
//! honestly be said about a group without inventing an edit that acts on all of
//! them.

mod controls;
mod edit;
#[cfg(test)]
mod fixture;
mod one;
mod selected;
mod several;
mod time;

use egui::{Color32, RichText, Ui};
use scorsese_core::Clip;

use crate::editing::Editing;
use crate::project::Open;
use edit::Refusal;
use selected::Selected;

/// The colour a refusal is said in — the same red as a project that will not
/// open, because it is the same kind of news.
const REFUSED: Color32 = Color32::from_rgb(220, 90, 80);

/// The panel's own state: nothing but the last thing it had to refuse.
///
/// Everything else it shows is read out of the document every frame. A panel
/// that kept its own copy of a clip would be a second edit that can disagree
/// with the first, and there is exactly one edit.
#[derive(Debug, Default)]
pub(crate) struct Inspector {
    refused: Option<Refusal>,
}

impl Inspector {
    /// Forgets a refusal, for when a different project is opened.
    pub(crate) fn reset(&mut self) {
        self.refused = None;
    }

    /// Draws the panel.
    pub(crate) fn show(&mut self, ui: &mut Ui, open: &mut Open, editing: &Editing) {
        ui.heading("Inspector");
        // A refusal belongs to the clip that earned it. Leaving that clip — for
        // another, for none, or for a group it is merely part of — is how it
        // gets dismissed, and coming back later must not re-open a complaint
        // that was read and dealt with.
        if self
            .refused
            .as_ref()
            .is_some_and(|last| Some(&last.clip) != editing.only())
        {
            self.refused = None;
        }

        match editing
            .only()
            .and_then(|clip| Selected::of(&open.project, clip))
        {
            Some(selected) => self.one(ui, open, &selected),
            None if editing.selected.len() > 1 => {
                several::show(ui, &open.project, &editing.selected);
            }
            None => nothing(ui),
        }
        self.refusal(ui);
    }

    /// Why the last change did not happen. Only ever about the clip on screen
    /// — [`Inspector::show`] drops a refusal the moment its clip is left.
    fn refusal(&self, ui: &mut Ui) {
        let Some(refused) = &self.refused else {
            return;
        };
        ui.add_space(10.0);
        ui.label(
            RichText::new(format!("{} unchanged", refused.what))
                .color(REFUSED)
                .strong()
                .small(),
        );
        for problem in &refused.problems {
            ui.label(RichText::new(format!("· {problem}")).color(REFUSED).small());
        }
    }

    /// Tries a change, and remembers why not when the project refuses it.
    fn attempt(
        &mut self,
        open: &mut Open,
        selected: &Selected,
        what: impl Into<String>,
        change: impl FnOnce(&mut Clip),
    ) {
        self.refused = edit::apply(open, &selected.clip, change)
            .err()
            .map(|problems| Refusal {
                clip: selected.clip.clone(),
                what: what.into(),
                problems,
            });
    }
}

/// Nothing selected — an invitation rather than an empty panel.
///
/// Also what a selection that outlived its clips comes to, which is why it is
/// the fall-through rather than a case of its own: a panel about a clip that
/// has gone has exactly as much to say as one about no clip at all.
fn nothing(ui: &mut Ui) {
    ui.add_space(4.0);
    ui.label(
        RichText::new("select a clip to see what it is")
            .weak()
            .italics(),
    );
}
