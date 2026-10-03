//! An image sequence's two plain values: how long each still is held, and
//! whether it loops.
//!
//! The only fields of a sequence a mouse is for. Which stills it plays, and in
//! what order, has structure to it — that is a sentence to an assistant, or
//! `scorsese sequence set` — so the panel says how many there are and leaves
//! the list alone.

use egui::{Checkbox, DragValue, Grid, RichText, Ui};
use scorsese_core::{AssetKind, Frames, ImageSequence};

use super::Inspector;
use super::selected::Selected;
use crate::project::Open;

impl Inspector {
    /// Draws the sequence's hold and loop under the clip's own fields, when the
    /// clip shows a sequence. On the **asset**, so every clip of it changes
    /// together — which the line under the heading says.
    pub(super) fn sequence(&mut self, ui: &mut Ui, open: &mut Open, selected: &Selected) {
        if selected.kind != Some(AssetKind::ImageSequence) {
            return;
        }
        let Some(sequence) = open
            .project
            .asset(&selected.asset)
            .and_then(|asset| asset.sequence.clone())
        else {
            return;
        };
        ui.add_space(10.0);
        ui.label(crate::theme::marks::subheading("Sequence"));
        ui.label(RichText::new(sequence.to_string()).weak().small());
        let mut change: Option<(&str, ImageSequence)> = None;
        Grid::new("sequence").num_columns(2).show(ui, |ui| {
            ui.label("Hold");
            let mut hold = sequence.hold.get();
            if ui
                .add(DragValue::new(&mut hold).range(1..=600).suffix(" f"))
                .on_hover_text("How many frames each still stays on screen")
                .changed()
            {
                let mut held = sequence.clone();
                held.hold = Frames(hold);
                change = Some(("hold", held));
            }
            ui.end_row();
            ui.label("Loop");
            let mut looping = sequence.looping;
            if ui
                .add(Checkbox::without_text(&mut looping))
                .on_hover_text("Start again from the first still, instead of holding the last")
                .changed()
            {
                let mut looped = sequence.clone();
                looped.looping = looping;
                change = Some(("loop", looped));
            }
            ui.end_row();
        });
        if let Some((what, sequence)) = change {
            let asset = selected.asset.clone();
            self.attempt_brief(open, selected, &asset, what, move |asset| {
                asset.sequence = Some(sequence);
            });
        }
    }
}
