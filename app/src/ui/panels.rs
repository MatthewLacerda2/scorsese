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
                // Beside the project's name rather than only over the picture,
                // because "why does nothing I click do anything" is a question
                // asked at the top of the window.
                let read_only = open.read_only();
                if read_only {
                    ui.label(
                        RichText::new("read-only")
                            .color(ui.visuals().warn_fg_color)
                            .strong(),
                    )
                    .on_hover_text(
                        "This project does not validate, so the window will not change it. \
                         What is wrong is listed in the centre.",
                    );
                }
                ui.separator();
                // In the bar rather than behind a menu, because the number it
                // leads to is one somebody checks before spending — and a shot
                // still generating is worth seeing without opening anything.
                //
                // Disabled rather than hidden on a read-only project: GO writes
                // assets and saves the document, and a button that vanishes
                // reads as a build that never had it.
                if ui
                    .add_enabled(!read_only, egui::Button::new("Generate…"))
                    .on_hover_text(
                        "What the unmade shots would cost, and the button that pays for them",
                    )
                    .on_disabled_hover_text("Not while this project does not validate")
                    .clicked()
                {
                    window.start_generating();
                }
                let generating = window.generating_count();
                if generating > 0 {
                    ui.label(
                        RichText::new(format!("{generating} generating"))
                            .weak()
                            .small(),
                    );
                }
            }
            // Said here rather than over the picture: an outside edit is
            // ordinary in this workflow, not an interruption, and the bar is
            // where a window says what it is showing.
            if let Some(note) = window.disk().note() {
                ui.separator();
                let text = RichText::new(note.text);
                ui.label(if note.trouble {
                    text.color(ui.visuals().warn_fg_color)
                } else {
                    text.weak()
                });
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
            // Asked before the borrow below, which takes the window mutably.
            let read_only = window.read_only();
            let Some((timeline, open, editing)) = window.timeline() else {
                ui.heading("Timeline");
                empty::placeholder(ui, "the tracks appear here");
                return;
            };
            // Drawn, and not interactive. A drag is an edit and every edit goes
            // straight to disk, so on a document that does not validate the
            // panel's job is to show which clip is the problem rather than to
            // let anyone move it.
            if read_only {
                ui.disable();
            }
            timeline.show(ui, open, editing);
        });
}

/// The inspector and project files, stacked down the right-hand edge.
pub(super) fn side(ui: &mut Ui, window: &mut Scorsese) {
    Panel::right("side")
        .default_size(SIDE_WIDTH)
        .show(ui, |ui| {
            let read_only = window.read_only();
            // The inspector is the one panel that changes the document, so it
            // is the one that most needs to be legible and inert at once: a
            // person repairing a project reads the offending clip's numbers
            // here and edits them in a text editor.
            match window.inspector() {
                Some((inspector, open, editing)) => {
                    // Scoped, so the files panel below stays live: listing the
                    // pool and highlighting an asset are reads, and taking them
                    // away would be protecting the document from being looked
                    // at.
                    ui.scope(|ui| {
                        if read_only {
                            ui.disable();
                        }
                        inspector.show(ui, open, editing);
                    });
                }
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
        // A document that does not validate gets this space instead of the
        // preview, and that is a claim about honesty rather than a limitation.
        // A frame composited from a project no renderer would accept is a
        // picture nobody can trust — and what the person opened the file for is
        // the list, not the picture.
        if let Some(open) = window.project().filter(|open| open.read_only()) {
            empty::invalid(ui, &open.problems);
            return;
        }
        let Some((preview, open, editing)) = window.preview() else {
            empty::nothing_open(ui, |window| window.pick(), window);
            return;
        };
        preview.show(ui, open, editing);
    });
}
