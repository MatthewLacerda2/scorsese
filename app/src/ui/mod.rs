//! The window: four panels, and what is in them.
//!
//! ```text
//! ┌──────────────────────────┬──────────────┐
//! │                          │  inspector   │
//! │         preview          ├──────────────┤
//! │                          │ project files│
//! ├──────────────────────────┴──────────────┤
//! │  ▏  timeline                            │
//! ├─────────────────────────────────────────┤
//! │ status                                  │
//! └─────────────────────────────────────────┘
//! ```
//!
//! The frame is here and the furniture is not: the preview, the timeline, the
//! inspector and the files list each arrive with their own issue. What this
//! module settles is where they go and who owns the project they all read.

mod disk;
mod empty;
mod keys;
mod panels;
mod status;

use eframe::{App, Frame};
use egui::Ui;
use scorsese_providers::credentials::Settings;

use crate::editing::Editing;
use crate::files::Files;
use crate::generating::Generating;
use crate::inspector::Inspector;
use crate::preview::Preview;
use crate::project::probing::{self, Probing};
use crate::project::watch::Watch;
use crate::project::{Open, Refused, open};
use crate::timeline::Timeline;
use disk::Disk;

/// The whole window's state.
pub struct Scorsese {
    /// The project, once one is open. `None` is the ordinary starting state,
    /// not an error — the window opens before anything is chosen.
    opened: Option<Open>,
    /// Why the last attempt failed, if it did. Cleared by a successful open.
    refused: Option<Refused>,
    /// Watches the open document for changes made outside this window.
    watch: Option<Watch>,
    /// Fills in the metadata of assets nobody has probed, on another thread.
    /// `None` once it has been collected, or when there was nothing to probe.
    probing: Option<Probing>,
    /// What the last such change turned out to be.
    disk: Disk,
    /// Where the window is looking: the playhead, and what is selected.
    editing: Editing,
    /// The timeline's own view — how far in, and how magnified.
    timeline: Timeline,
    /// The pool as the files panel last read it.
    files: Files,
    /// The inspector, which is the one panel that changes the document.
    inspector: Inspector,
    /// The picture at the playhead, and the transport under it.
    preview: Preview,
    /// The generate dialog, and whatever it has running.
    generating: Generating,
    /// What this machine knows and no project does: the keys, and the ceiling
    /// on what one run may spend.
    ///
    /// Held rather than read where it is needed, because it is the one thing
    /// this window draws that comes from neither the document nor the person at
    /// the keyboard. A panel that called `Settings::load` drew a different
    /// sentence for every machine, and a reference image of it was a picture of
    /// whoever last ran the test — see [`Scorsese::opening_with`].
    settings: Settings,
}

impl Scorsese {
    /// A window, optionally starting on a directory given on the command line,
    /// reading this machine's settings.
    pub fn opening(directory: Option<std::path::PathBuf>) -> Self {
        Self::opening_with(directory, Settings::load().unwrap_or_default())
    }

    /// The same window, told what machine it is on instead of asking.
    ///
    /// **What the tests open with.** The generate dialog reads a ceiling, and
    /// `~/.config/scorsese/settings.json` is where one lives — so a snapshot
    /// drawn through [`Scorsese::opening`] is a test of the developer's own
    /// configuration, green for a runner that has none and red for anybody who
    /// has ever set one. Stating the settings is the only thing an environment
    /// variable cannot get in front of: there is no file to point at.
    pub fn opening_with(directory: Option<std::path::PathBuf>, settings: Settings) -> Self {
        let mut window = Self {
            opened: None,
            refused: None,
            watch: None,
            probing: None,
            disk: Disk::default(),
            editing: Editing::default(),
            timeline: Timeline::default(),
            files: Files::default(),
            inspector: Inspector::default(),
            preview: Preview::default(),
            generating: Generating::default(),
            settings,
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
                self.watch = Some(Watch::on(directory));
                self.disk = Disk::Quiet;
                self.opened = Some(project);
                self.refused = None;
                // A playhead left at frame 900 of the last film, or a view
                // fitted to its length, is worse than starting over.
                self.editing.reset();
                self.timeline.reset();
                self.files.reset();
                self.inspector.reset();
                // A frame of the last film left on screen under the new one's
                // playhead would be the preview lying on its first repaint.
                self.preview.reset();
                self.generating.reset();
                if let Some(open) = &self.opened {
                    self.files.refresh(open);
                    // Started here and nowhere else: opening is the one moment
                    // a whole pool arrives at once, and it is the moment
                    // nobody is holding a clip.
                    //
                    // Not started at all on a read-only project, and that is
                    // the sharpest edge of this rather than a tidy-up: the
                    // probe records what it learns and saves, so on a document
                    // that already does not validate it would write the broken
                    // state back out — and a length arriving late is how a
                    // project gets into this state to begin with.
                    self.probing = (!open.read_only())
                        .then(|| Probing::start(&open.project, &open.root))
                        .flatten();
                }
            }
            Err(refused) => self.refused = Some(refused),
        }
    }

    /// Records what the background probe found, if it has finished.
    ///
    /// The findings are applied to the document the window holds now and saved
    /// straight away, because the document on disk is the model — metadata the
    /// window knows and the file does not would be the one divergence this app
    /// is not allowed to have. A save that fails is not reported: nobody asked
    /// for this, and a read-only project is not a thing to interrupt someone
    /// about.
    fn follow_probe(&mut self) {
        let Some(found) = self.probing.as_mut().and_then(Probing::finished) else {
            return;
        };
        self.probing = None;
        let Some(open) = &mut self.opened else {
            return;
        };
        // Said twice on purpose. Nothing starts a probe on a read-only project,
        // and nothing writes one either — a guard that only exists at the point
        // work is started is a guard that lasts until somebody starts it
        // somewhere else.
        if open.read_only() {
            return;
        }
        if probing::record(&mut open.project, &found) {
            let _ = open.project.save(&open.root);
            // What was just learned can be what makes the document stop
            // validating — a clip reaching past media nobody had measured is
            // not wrong until somebody measures it — so the answer is asked
            // again here. Without this the window would carry on offering to
            // edit a document it has just made invalid, and every drag would be
            // refused over some clip other than the one under the pointer.
            open.revalidate();
            self.files.refresh(open);
        }
    }

    /// Opens the generate dialog.
    ///
    /// `pub` for the same reason [`Scorsese::select`] is: a snapshot test
    /// drives the window through its own surface rather than reaching into it,
    /// and a dialog nothing can open is a dialog nobody has looked at.
    pub fn start_generating(&mut self) {
        self.generating.open();
    }

    /// How many shots are in flight, for the bar to say so.
    pub(crate) fn generating_count(&self) -> usize {
        self.generating.in_flight()
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

impl Scorsese {
    /// Points the window at a clip, as clicking it in the timeline does —
    /// replacing whatever was selected before.
    ///
    /// Public for two reasons, and the second is worth stating plainly. A
    /// window has more than one way to point at something — the files panel
    /// already highlights an asset — so selecting from outside the timeline is
    /// a real operation. And it is what lets a snapshot put the inspector into
    /// the state it exists for, without synthesising a pointer event at a pixel
    /// computed from the layout it is supposed to be checking.
    pub fn select(&mut self, clip: &str) {
        self.editing.pick(&scorsese_core::ClipId::new(clip), false);
    }

    /// Adds a clip to the selection, as shift-clicking it does.
    ///
    /// The second half of [`Scorsese::select`]'s second reason: a window with
    /// three clips selected is a state worth looking at, and reaching it
    /// through synthesised modifier-clicks at computed pixels would be a test
    /// of the layout rather than of the picture.
    pub fn also_select(&mut self, clip: &str) {
        self.editing.pick(&scorsese_core::ClipId::new(clip), true);
    }

    /// Points the window at an instant, as dragging the scrubber does.
    ///
    /// Public for the second of [`Scorsese::select`]'s two reasons: putting
    /// the playhead somewhere is otherwise only possible by synthesising a
    /// pointer drag at a pixel computed from the layout, and "the playhead did
    /// not move when the document was re-read" is a thing worth asserting
    /// directly.
    pub fn scrub(&mut self, to: scorsese_core::Frames) {
        self.editing.playhead = to;
    }

    /// The document the window is showing, if one is open.
    ///
    /// Read-only and deliberately narrow, and public for the same reason
    /// [`Scorsese::select`] is: a window whose state can only be observed as
    /// pixels can only be asserted about by looking at a picture, and "the
    /// document was re-read and the playhead did not move" is not something a
    /// picture says clearly.
    pub fn showing(&self) -> Option<&scorsese_core::Project> {
        self.opened.as_ref().map(|open| &open.project)
    }

    /// Where the playhead is standing.
    pub fn playhead(&self) -> scorsese_core::Frames {
        self.editing.playhead
    }

    /// Which clips are selected, in the selection's own order.
    pub fn selected(&self) -> Vec<&scorsese_core::ClipId> {
        self.editing.selected.iter().collect()
    }

    /// What the window would say about the document on disk — the same
    /// sentence the bar shows, or nothing when it has nothing to say.
    pub fn disk_note(&self) -> Option<String> {
        self.disk.note().map(|note| note.text)
    }

    /// Draws the whole window into `ui`.
    ///
    /// Takes a `Ui` and nothing else — no `eframe::Frame`, no event loop —
    /// which is the whole reason it is a method rather than the body of
    /// [`App::ui`]. A window that can only be drawn by an event loop is a
    /// window nobody can look at in a test, and six panels were built that way
    /// before this existed.
    pub fn draw(&mut self, ui: &mut Ui) {
        // Here and not in `main.rs`, so that the offscreen harness draws the
        // same window a person sees. See [`crate::theme`].
        crate::theme::apply(ui.ctx());
        self.follow_disk(ui.ctx());
        self.follow_probe();
        // Before the panels, so a key pressed this frame moves the playhead
        // that this frame's timeline and preview are drawn from — read after
        // them and every keypress would show up one frame late, which at a held
        // arrow key is a scrub that lags the hand.
        self.follow_keyboard(ui);
        // Polled every repaint and blocking on nothing: a submit or a sweep
        // that has finished is folded in here, and one still going leaves the
        // window exactly as responsive as it was.
        if let Some(open) = &mut self.opened {
            self.generating.poll(open);
        }
        // Declared outside-in, which is what egui's layout wants: each panel
        // takes its edge and leaves the rest to the next. The centre goes last
        // and gets whatever is left.
        panels::menu(ui, self);
        panels::status(ui, self);
        panels::timeline(ui, self);
        panels::side(ui, self);
        panels::centre(ui, self);
        // Last, and floating: a dialog belongs over the panels rather than
        // taking an edge from them.
        if let Some(open) = &mut self.opened {
            self.generating.show(ui.ctx(), open, &mut self.settings);
        }
    }
}

impl App for Scorsese {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        self.draw(ui);
    }
}

impl Scorsese {
    /// The project, for the panels that draw it.
    pub(crate) fn project(&self) -> Option<&Open> {
        self.opened.as_ref()
    }

    /// Whether what is open may be changed.
    ///
    /// False when nothing is open, because there is nothing to protect — the
    /// panels ask this to decide whether to accept interaction, and an empty
    /// window has none to accept.
    pub(crate) fn read_only(&self) -> bool {
        self.opened.as_ref().is_some_and(Open::read_only)
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
    ///
    /// The document goes out **mutably**: dragging a clip is an edit, and the
    /// document on disk is the only model there is to make it in.
    pub(crate) fn timeline(&mut self) -> Option<(&mut Timeline, &mut Open, &mut Editing)> {
        let opened = self.opened.as_mut()?;
        self.editing.forget_missing(&opened.project);
        Some((&mut self.timeline, opened, &mut self.editing))
    }

    /// The preview, the document it draws a frame of, and the playhead it
    /// moves — handed out together for the same reason the timeline's three
    /// are: it needs all of them at once, and a caller cannot collect them one
    /// at a time.
    pub(crate) fn preview(&mut self) -> Option<(&mut Preview, &Open, &mut Editing)> {
        let opened = self.opened.as_ref()?;
        Some((&mut self.preview, opened, &mut self.editing))
    }

    /// Why the last open failed, for the panel that has to say so.
    pub(crate) fn problem(&self) -> Option<&Refused> {
        self.refused.as_ref()
    }

    /// What the document on disk last did, for the bar that reports it.
    pub(crate) fn disk(&self) -> &Disk {
        &self.disk
    }
}
