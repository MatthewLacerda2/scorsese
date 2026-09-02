//! Where each panel sits, and the two strips that bracket them.

use egui::{CentralPanel, Frame, Margin, Panel, RichText, Ui};

use super::Scorsese;
use super::{empty, status};
use crate::theme::{marks, palette};

/// How tall the timeline strip opens when there is no project to size it to.
const EMPTY_TIMELINE_HEIGHT: f32 = 140.0;
/// How wide the inspector and files column opens.
const SIDE_WIDTH: f32 = 296.0;

/// The bar along the top: the wordmark, what is open, and what may be done to
/// it that is not an edit.
///
/// The name is on it because a window with no title bar of its own — which is
/// what a maximised app on most desktops is — has nowhere else to say what
/// program you are in. Set in the same letterspaced capitals as a section
/// heading, because that is the one typographic idea this look has and using it
/// for the application's own name is where it should start.
pub(super) fn menu(ui: &mut Ui, window: &mut Scorsese) {
    Panel::top("menu")
        .frame(chrome(Margin::symmetric(10, 5)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("SCORSESE")
                        .strong()
                        .extra_letter_spacing(3.0)
                        .color(palette::ACCENT),
                );
                ui.add_space(4.0);
                if ui.button("Open…").clicked() {
                    window.pick();
                }
                opened(ui, window);
                // Said here rather than over the picture: an outside edit is
                // ordinary in this workflow, not an interruption, and the bar is
                // where a window says what it is showing.
                if let Some(note) = window.disk().note() {
                    ui.add_space(6.0);
                    let text = RichText::new(note.text).small();
                    ui.label(if note.trouble {
                        text.color(palette::ALERT)
                    } else {
                        text.color(palette::DIM)
                    });
                }
            });
        });
}

/// Everything on the bar that needs a project to be about.
fn opened(ui: &mut Ui, window: &mut Scorsese) {
    let Some(open) = window.project() else {
        return;
    };
    let read_only = open.read_only();
    ui.add_space(4.0);
    ui.label(RichText::new(open.directory()).strong());
    // Beside the project's name rather than only over the picture, because
    // "why does nothing I click do anything" is a question asked at the top of
    // the window.
    if read_only {
        ui.label(
            RichText::new("READ-ONLY")
                .small()
                .strong()
                .extra_letter_spacing(1.0)
                .color(palette::WARM),
        )
        .on_hover_text(
            "This project does not validate, so the window will not change it. \
             What is wrong is listed in the centre.",
        );
    }
    ui.add_space(4.0);
    // On the bar rather than behind a menu, because the number it leads to is
    // one somebody checks before spending — and a shot still generating is
    // worth seeing without opening anything.
    //
    // Disabled rather than hidden on a read-only project: GO writes assets and
    // saves the document, and a button that vanishes reads as a build that
    // never had it.
    if ui
        .add_enabled(!read_only, egui::Button::new("Generate…"))
        .on_hover_text("What the unmade shots would cost, and the button that pays for them")
        .on_disabled_hover_text("Not while this project does not validate")
        .clicked()
    {
        window.start_generating();
    }
    let generating = window.generating_count();
    if generating > 0 {
        ui.label(
            RichText::new(format!("{generating} generating"))
                .small()
                .color(palette::ACCENT),
        );
    }
}

/// The status strip, along the very bottom and under the timeline.
///
/// Declared before the timeline so that it takes the window's own bottom edge:
/// `egui` lays panels out from the outside in, and a reading that moved up and
/// down as the timeline was resized would be one nobody could find twice.
pub(super) fn status(ui: &mut Ui, window: &Scorsese) {
    Panel::bottom("status")
        .frame(chrome(Margin::symmetric(10, 3)))
        .show(ui, |ui| status::show(ui, window));
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
        .frame(Frame::NONE.fill(palette::INK))
        .show(ui, |ui| {
            // Asked before the borrow below, which takes the window mutably.
            let read_only = window.read_only();
            let Some((timeline, open, editing)) = window.timeline() else {
                ui.add_space(6.0);
                ui.indent("empty timeline", |ui| {
                    marks::section(ui, "Timeline");
                    empty::placeholder(ui, "the tracks appear here");
                });
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
        // A floor and not only a starting width. `Panel` sizes itself to its
        // content unless the content takes the available space, and none of
        // what is in here does — so without this the column is as narrow as its
        // longest sentence, which on an empty inspector is narrow enough to
        // wrap *"select a clip to see what it is"* onto two lines.
        .min_size(SIDE_WIDTH)
        .frame(chrome(Margin::symmetric(10, 6)))
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
                    marks::section(ui, "Inspector");
                    empty::placeholder(ui, "select a clip to see what it is");
                }
            }
            ui.add_space(10.0);

            let Some((files, open, editing)) = window.files() else {
                marks::section(ui, "Project files");
                empty::placeholder(ui, "the assets appear here");
                return;
            };
            files.show(ui, open, editing);
        });
}

/// The preview, and — when nothing is open or something went wrong — whatever
/// has to be said instead.
pub(super) fn centre(ui: &mut Ui, window: &mut Scorsese) {
    CentralPanel::default()
        .frame(Frame::NONE.fill(palette::VOID))
        .show(ui, |ui| {
            if let Some(refused) = window.problem() {
                empty::refusal(ui, refused);
                return;
            }
            // A document that does not validate gets this space instead of the
            // preview, and that is a claim about honesty rather than a
            // limitation. A frame composited from a project no renderer would
            // accept is a picture nobody can trust — and what the person opened
            // the file for is the list, not the picture.
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

/// The frame a chrome panel wears: the panel fill, a hairline along the edge it
/// shares with the middle of the window, and the given breathing room.
///
/// One function rather than four, so the bar at the top and the strip at the
/// bottom cannot end up a pixel apart from each other by nobody deciding.
fn chrome(margin: Margin) -> Frame {
    Frame::NONE
        .fill(palette::INK)
        .stroke(egui::Stroke::new(1.0, palette::RULE))
        .inner_margin(margin)
}
