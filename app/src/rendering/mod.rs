//! Rendering the edit to a file from the window: the settings, the save
//! dialog, and the popup that follows the render to the end.
//!
//! **Not an editing operation, and still a mouse one.** Exporting is the most
//! ordinary thing a person does in an editor, and before this a desktop user
//! had to leave the window for the CLI or an assistant to get a video out. It
//! adds no capability: it is `scorsese render` given a button, calling the same
//! [`scorsese_render::Renderer`] with the same defaults.
//!
//! **The window stays usable, editing included.** The render runs on its own
//! thread from a copy of the document taken when it starts ([`job`]), so
//! scrubbing, selecting and editing all carry on, and nothing done meanwhile
//! can reach the file being written — it is the edit as it stood when Render
//! was pressed. Blocking edits instead would freeze the one thing people do
//! while they wait, to protect against a hazard the copy already removes.

mod choices;
mod job;
mod popup;

use std::path::PathBuf;

use egui::{Context, Window};
use scorsese_render::Reading;

use crate::project::Open;
use choices::Choices;
use job::Job;

/// The render dialog, and the render it started.
#[derive(Default)]
pub(crate) struct Rendering {
    /// Whether the settings dialog is open.
    asking: bool,
    /// What it was last set to, kept between renders so a second one starts
    /// where the first left off.
    choices: Choices,
    /// The render running, or finished and not yet dismissed.
    job: Option<Job>,
}

impl Rendering {
    /// Opens the settings dialog.
    pub(crate) fn ask(&mut self) {
        self.asking = true;
    }

    /// Whether a render is under way — the bar's button waits for it.
    pub(crate) fn busy(&self) -> bool {
        self.job.as_ref().is_some_and(Job::running)
    }

    /// Shows the popup at a fixed reading, as if a render to `out` were that
    /// far along — what a snapshot test draws.
    pub(crate) fn hold(&mut self, reading: Reading, out: PathBuf) {
        self.asking = false;
        self.job = Some(Job::held(reading, out));
    }

    /// Draws the dialog and the popup, whichever is up. `open` is only read:
    /// a render takes a copy of it and never writes it back.
    pub(crate) fn show(&mut self, ctx: &Context, open: Option<&Open>) {
        if let Some(job) = &mut self.job
            && popup::show(ctx, job)
        {
            self.job = None;
        }
        // The dialog needs a document to render; closing the project closes it.
        let Some(open) = open else {
            self.asking = false;
            return;
        };
        if self.asking && self.job.is_none() {
            self.dialog(ctx, open);
        }
    }

    /// The settings and the button that asks where to save.
    fn dialog(&mut self, ctx: &Context, open: &Open) {
        let own = open.project.timeline_fps;
        let mut showing = true;
        let mut start = false;
        Window::new("Render")
            .open(&mut showing)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                self.choices.show(ui, own);
                ui.separator();
                start = ui.button("Render…").clicked();
            });
        self.asking = showing && !start;
        if start && let Some(out) = self.destination(open) {
            let settings = self.choices.settings(own);
            self.job = Some(Job::start(&open.project, &open.root, settings, out));
        }
    }

    /// Asks where the file goes. `None` when the dialog was cancelled.
    ///
    /// The extension is the chosen file's, whatever was typed: the menu is
    /// where the kind of file was decided, and a name ending otherwise would be
    /// a file that says it is something it is not.
    fn destination(&self, open: &Open) -> Option<PathBuf> {
        let container = self.choices.container();
        let extension = container.name();
        let mut out = rfd::FileDialog::new()
            .set_title("Render to")
            .set_file_name(format!("{}.{extension}", stem(open)))
            .add_filter(choices::kind(container), &[extension])
            .save_file()?;
        out.set_extension(extension);
        Some(out)
    }
}

/// What the file is called unless somebody types otherwise: the project's own
/// name, or its folder's when it has none.
fn stem(open: &Open) -> String {
    let name = open.project.name.trim();
    if name.is_empty() {
        open.directory().trim_end_matches(".scor").to_owned()
    } else {
        name.to_owned()
    }
}
