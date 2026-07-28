//! The window: four panels, and what is in them.
//!
//! ```text
//! ┌──────────────────────────┬──────────────┐
//! │                          │  inspector   │
//! │         preview          ├──────────────┤
//! │                          │ project files│
//! ├──────────────────────────┴──────────────┤
//! │  ▏  timeline                            │
//! └─────────────────────────────────────────┘
//! ```
//!
//! The frame is here and the furniture is not: the preview, the timeline, the
//! inspector and the files list each arrive with their own issue. What this
//! module settles is where they go and who owns the project they all read.

mod empty;
mod panels;

use eframe::{App, Frame};
use egui::Ui;

use crate::project::{Open, Refused, open};

/// The whole window's state.
pub(crate) struct Scorsese {
    /// The project, once one is open. `None` is the ordinary starting state,
    /// not an error — the window opens before anything is chosen.
    opened: Option<Open>,
    /// Why the last attempt failed, if it did. Cleared by a successful open.
    refused: Option<Refused>,
}

impl Scorsese {
    /// A window, optionally starting on a directory given on the command line.
    pub(crate) fn opening(directory: Option<std::path::PathBuf>) -> Self {
        let mut window = Self {
            opened: None,
            refused: None,
        };
        if let Some(directory) = directory {
            window.open(&directory);
        }
        window
    }

    /// Opens a directory, replacing whatever was open.
    ///
    /// A failure leaves the previous project alone: losing what you were
    /// working on because you mis-clicked a folder would be the worst possible
    /// answer to a mis-click.
    fn open(&mut self, directory: &std::path::Path) {
        match open(directory) {
            Ok(project) => {
                self.opened = Some(project);
                self.refused = None;
            }
            Err(refused) => self.refused = Some(refused),
        }
    }

    /// Asks for a directory and opens it. Cancelling does nothing at all.
    pub(crate) fn pick(&mut self) {
        if let Some(directory) = rfd::FileDialog::new()
            .set_title("Open a scorsese project")
            .pick_folder()
        {
            self.open(&directory);
        }
    }
}

impl App for Scorsese {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        // Declared outside-in, which is what egui's layout wants: each panel
        // takes its edge and leaves the rest to the next. The centre goes last
        // and gets whatever is left.
        panels::menu(ui, self);
        panels::timeline(ui, self.opened.as_ref());
        panels::side(ui, self.opened.as_ref());
        panels::centre(ui, self);
    }
}

impl Scorsese {
    /// The project, for the panels that draw it.
    pub(crate) fn project(&self) -> Option<&Open> {
        self.opened.as_ref()
    }

    /// Why the last open failed, for the panel that has to say so.
    pub(crate) fn problem(&self) -> Option<&Refused> {
        self.refused.as_ref()
    }
}
