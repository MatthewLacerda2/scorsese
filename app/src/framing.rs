//! The frame's shape: one plain control on the bar (#1403).
//!
//! The project carries its shape (`project.json`'s `aspect`, #1278), and the
//! preview, a kept frame and the Render dialog's sizes all follow it. Changing
//! it is the operation a person reaches for with a mouse — 16:9 to 9:16 to
//! post a cut upright — so it is a dropdown here rather than a sentence to an
//! assistant. It runs [`scorsese_render::reframe`], the same re-fit `scorsese
//! reframe` and the `project_reframe` tool make, so footage that filled the old
//! frame fills the new one; what that cannot settle is said beside the
//! control, for the assistant or a hand on the inspector to fix.

use egui::{RichText, Ui};
use scorsese_core::Ratio;
use scorsese_render::reframe;

use crate::project::Open;

/// The shapes on offer, `across:down`: the ones a person with an idea reaches
/// for — the web editor's four.
const SHAPES: [&str; 4] = ["16:9", "9:16", "1:1", "4:5"];

/// What the last change of shape came to, while it is worth saying.
#[derive(Debug, Default)]
pub(crate) struct Framing {
    /// The re-fit's report, said beside the control until another project is
    /// opened or it is dismissed.
    said: Option<Said>,
}

/// One change's report: its first line on the bar, every line on hover.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Said {
    headline: String,
    whole: String,
    /// How many things were left to settle — counted in the headline, since
    /// the bar has room for one line.
    unsettled: usize,
}

impl Framing {
    /// Forgets the last report, for when another project is opened.
    pub(crate) fn reset(&mut self) {
        self.said = None;
    }

    /// Draws the picker and the last report; a shape chosen is re-fitted and
    /// saved. `enabled` is false on a project that does not validate, which
    /// the window never changes.
    pub(crate) fn show(&mut self, ui: &mut Ui, open: &mut Open, enabled: bool) {
        let now = open.project.aspect;
        let mut chosen = now;
        // A menu off a button rather than a combo box: the bar's other
        // controls are buttons, and a combo box's taller frame would grow the
        // bar under every panel.
        ui.add_enabled_ui(enabled, |ui| {
            ui.menu_button(format!("{now} ⏷"), |ui| {
                for shape in offered(now) {
                    if ui
                        .selectable_label(shape == now, shape.to_string())
                        .clicked()
                    {
                        chosen = shape;
                        ui.close();
                    }
                }
            })
            .response
            .on_hover_text("The frame's shape: the preview and every render follow it")
            .on_disabled_hover_text("Not while this project does not validate");
        });
        if chosen != now {
            self.said = Some(change(open, chosen));
        }
        let Some(said) = &self.said else {
            return;
        };
        let mut text = said.headline.clone();
        if said.unsettled > 0 {
            text.push_str(&format!(" — {} left to settle", said.unsettled));
        }
        let response = ui
            .label(RichText::new(text).small().weak())
            .on_hover_text(&said.whole);
        if ui.small_button("✕").on_hover_text("Dismiss").clicked() || response.clicked() {
            self.said = None;
        }
    }
}

/// The shapes to offer while the project is `now`: the usual four, and its own
/// if it is none of them, so the control can show what the project is.
fn offered(now: Ratio) -> Vec<Ratio> {
    let mut shapes: Vec<Ratio> = SHAPES
        .iter()
        .filter_map(|shape| shape.parse().ok())
        .collect();
    if !shapes.contains(&now) {
        shapes.push(now);
    }
    shapes
}

/// Re-fits `open` to `to` on a copy and saves it, keeping it only once it is
/// on disk — the document on disk is the model (`inspector::edit`).
fn change(open: &mut Open, to: Ratio) -> Said {
    let mut candidate = open.project.clone();
    let reframed = reframe(&mut candidate, &open.root, to);
    let whole = reframed.to_string();
    let headline = whole.lines().next().unwrap_or_default().to_owned();
    let unsettled = reframed.unsettled.len();
    if reframed.refit.changed()
        && let Err(why) = candidate.save(&open.root)
    {
        let whole = format!("The frame stays {}: {why}", open.project.aspect);
        return Said {
            headline: whole.clone(),
            whole,
            unsettled: 0,
        };
    }
    open.project = candidate;
    Said {
        headline,
        whole,
        unsettled,
    }
}

#[cfg(test)]
mod tests;
