//! What a panel says when it has nothing to draw.
//!
//! Every one of these is a real state a person will spend time in — the window
//! before anything is opened, a panel before its issue lands, a folder that
//! turned out not to be a project. Saying nothing in any of them would read as
//! the app being broken, which is a worse first impression than saying "not
//! yet".

use egui::{Color32, RichText, Ui};

use crate::project::Refused;

/// A quiet line standing in for something that is not built yet, or has
/// nothing in it. Dimmed, so it never reads as content.
pub(super) fn placeholder(ui: &mut Ui, what: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(what).weak().italics());
}

/// The window with no project open — the first thing anyone sees.
///
/// It carries the open action itself rather than only pointing at the menu: an
/// empty window whose one affordance is hidden in a menu is a window people
/// close again.
pub(super) fn nothing_open<T>(ui: &mut Ui, open: impl FnOnce(&mut T), window: &mut T) {
    let mut clicked = false;
    ui.vertical_centered(|ui| {
        ui.add_space(80.0);
        ui.heading("No project open");
        ui.add_space(8.0);
        ui.label("Open a *.scor directory to get started.");
        ui.add_space(16.0);
        clicked = ui.button("Open project…").clicked();
    });
    if clicked {
        open(window);
    }
}

/// Why a directory could not be opened, with every problem listed.
///
/// All of them, never just the first — a person repairing a project should be
/// able to see the whole job, which is the promise validation already makes to
/// the CLI.
pub(super) fn refusal(ui: &mut Ui, refused: &Refused) {
    listing(ui, &refused.heading, &refused.problems);
}

/// What is wrong with a project that **did** open, in the space the preview
/// would have had.
///
/// The same list as a refusal, and deliberately the same shape. The two used to
/// be one state from the outside — "does not validate" and "is not a project"
/// both meant a window with nothing in it — and what separates them now is
/// everything *else* on the screen rather than anything in this panel.
pub(super) fn invalid(ui: &mut Ui, problems: &[String]) {
    listing(ui, &crate::project::heading(problems.len()), problems);
    ui.add_space(12.0);
    ui.label(
        RichText::new(
            "The timeline, the pool and the inspector are here to read. \
             Nothing in this window will change the file until these are fixed.",
        )
        .weak(),
    );
}

/// A heading and every problem under it, one to a line.
fn listing(ui: &mut Ui, heading: &str, problems: &[String]) {
    ui.add_space(24.0);
    ui.heading(RichText::new(heading).color(Color32::from_rgb(220, 90, 80)));
    ui.add_space(8.0);
    egui::ScrollArea::vertical().show(ui, |ui| {
        for problem in problems {
            ui.label(format!("· {problem}"));
        }
    });
}
