//! Where each panel sits.

use egui::{CentralPanel, MenuBar, Panel, RichText, Ui};

use super::Scorsese;
use super::empty;

/// How tall the timeline strip opens when there is no project to size it to.
const EMPTY_TIMELINE_HEIGHT: f32 = 140.0;
/// How wide the inspector and files column opens.
const SIDE_WIDTH: f32 = 280.0;

/// The menu bar: opening a project, and nothing else yet.
pub(super) fn menu(ui: &mut Ui, window: &mut Scorsese) {
    Panel::top("menu").show(ui, |ui| {
        MenuBar::new().ui(ui, |ui| {
            if ui.button("Open project…").clicked() {
                window.pick();
            }
            if let Some(open) = window.project() {
                ui.separator();
                ui.label(RichText::new(open.directory()).strong());
            }
        });
    });
}

/// The timeline strip along the bottom.
pub(super) fn timeline(ui: &mut Ui, window: &mut Scorsese) {
    let wanted = window.project().map_or(EMPTY_TIMELINE_HEIGHT, |open| {
        crate::timeline::desired_height(&open.project)
    });
    Panel::bottom("timeline")
        .default_size(wanted)
        .min_size(100.0)
        .resizable(true)
        .show(ui, |ui| {
            let Some((timeline, open, editing)) = window.timeline() else {
                ui.heading("Timeline");
                empty::placeholder(ui, "the tracks appear here");
                return;
            };
            timeline.show(ui, &open.project, editing);
        });
}

/// The inspector and project files, stacked down the right-hand edge.
pub(super) fn side(ui: &mut Ui, window: &mut Scorsese) {
    Panel::right("side")
        .default_size(SIDE_WIDTH)
        .show(ui, |ui| {
            match window.inspector() {
                Some((inspector, open, editing)) => inspector.show(ui, open, editing),
                None => {
                    ui.heading("Inspector");
                    empty::placeholder(ui, "select a clip to see what it is");
                }
            }
            ui.separator();

            let Some((files, open, editing)) = window.files() else {
                ui.heading("Project files");
                empty::placeholder(ui, "the assets appear here");
                return;
            };
            files.show(ui, open, editing);
        });
}

/// The preview, and — when nothing is open or something went wrong — whatever
/// has to be said instead.
pub(super) fn centre(ui: &mut Ui, window: &mut Scorsese) {
    CentralPanel::default().show(ui, |ui| {
        if let Some(refused) = window.problem() {
            empty::refusal(ui, refused);
            return;
        }
        let Some((preview, open, editing)) = window.preview() else {
            empty::nothing_open(ui, |window| window.pick(), window);
            return;
        };
        preview.show(ui, open, editing);
    });
}
