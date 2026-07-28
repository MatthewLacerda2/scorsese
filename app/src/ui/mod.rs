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

use crate::editing::Editing;
use crate::files::Files;
use crate::inspector::Inspector;
use crate::project::{Open, Refused, open};
use crate::timeline::Timeline;

/// The whole window's state.
pub(crate) struct Scorsese {
    /// The project, once one is open. `None` is the ordinary starting state,
    /// not an error — the window opens before anything is chosen.
    opened: Option<Open>,
    /// Why the last attempt failed, if it did. Cleared by a successful open.
    refused: Option<Refused>,
    /// Where the window is looking: the playhead, and what is selected.
    editing: Editing,
    /// The timeline's own view — how far in, and how magnified.
    timeline: Timeline,
    /// The pool as the files panel last read it.
    files: Files,
    /// The inspector, which is the one panel that changes the document.
    inspector: Inspector,
}

impl Scorsese {
    /// A window, optionally starting on a directory given on the command line.
    pub(crate) fn opening(directory: Option<std::path::PathBuf>) -> Self {
        let mut window = Self {
            opened: None,
            refused: None,
            editing: Editing::default(),
            timeline: Timeline::default(),
            files: Files::default(),
            inspector: Inspector::default(),
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
                // A playhead left at frame 900 of the last film, or a view
                // fitted to its length, is worse than starting over.
                self.editing.reset();
                self.timeline.reset();
                self.files.reset();
                self.inspector.reset();
                if let Some(open) = &self.opened {
                    self.files.refresh(open);
                }
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
        panels::timeline(ui, self);
        panels::side(ui, self);
        panels::centre(ui, self);
    }
}

impl Scorsese {
    /// The project, for the panels that draw it.
    pub(crate) fn project(&self) -> Option<&Open> {
        self.opened.as_ref()
    }

    /// The files panel, the project it lists, and the view state it sets.
    pub(crate) fn files(&mut self) -> Option<(&mut Files, &Open, &mut Editing)> {
        let opened = self.opened.as_ref()?;
        Some((&mut self.files, opened, &mut self.editing))
    }

    /// The inspector, the document it edits, and what is selected in it.
    ///
    /// The project by `&mut`, alone among the panels: this is the one that
    /// changes it. The selection is read-only here — the timeline is where a
    /// clip is picked, and two panels able to set it would be two answers to
    /// one question.
    pub(crate) fn inspector(&mut self) -> Option<(&mut Inspector, &mut Open, &Editing)> {
        let opened = self.opened.as_mut()?;
        Some((&mut self.inspector, opened, &self.editing))
    }

    /// The timeline, the document it draws, and the view state it moves —
    /// handed out together because it needs all three at once and the
    /// borrow checker will not let a caller collect them one at a time.
    pub(crate) fn timeline(&mut self) -> Option<(&mut Timeline, &Open, &mut Editing)> {
        let opened = self.opened.as_ref()?;
        self.editing.forget_missing(&opened.project);
        Some((&mut self.timeline, opened, &mut self.editing))
    }

    /// Why the last open failed, for the panel that has to say so.
    pub(crate) fn problem(&self) -> Option<&Refused> {
        self.refused.as_ref()
    }
}
