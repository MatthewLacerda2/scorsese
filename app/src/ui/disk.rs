//! Following the document while something else edits it.
//!
//! The window is one of two writers on one file, and the *other* one is the
//! point: an assistant does the structural editing over MCP while a person
//! watches. What lives here is everything about noticing that, and the rule
//! for what happens when both want the document at once — which is stated in
//! [`crate::project::watch`] and enforced in [`Scorsese::follow_disk`].

use crate::project::watch;
use crate::project::{Refused, open};

use super::Scorsese;

/// What the last look at the disk found.
///
/// Separate from [`Scorsese::problem`], which is about *opening* something.
/// This is about the thing already open changing underneath, and the two want
/// opposite treatment: a directory that will not open leaves an empty window,
/// while a document that will not re-load leaves the last good one on screen.
#[derive(Default)]
pub(crate) enum Disk {
    /// Nothing has changed under the window.
    #[default]
    Quiet,
    /// The document changed and was re-read.
    Reloaded,
    /// It changed into something that will not load. What is on screen is the
    /// last version that did.
    Unreadable(Refused),
}

/// One line about the document on disk, for the bar to show.
pub(crate) struct Note {
    /// What to say.
    pub(crate) text: String,
    /// Whether it is something gone wrong, as opposed to something that
    /// merely happened.
    pub(crate) trouble: bool,
}

impl Disk {
    /// What to say about the document on disk, or nothing when there is
    /// nothing to say.
    ///
    /// The words live here rather than in the panel so that the interface and
    /// anything asking the window what it would show read the same sentence.
    pub(crate) fn note(&self) -> Option<Note> {
        match self {
            Self::Quiet => None,
            Self::Reloaded => Some(Note {
                text: "reloaded — changed on disk".to_owned(),
                trouble: false,
            }),
            Self::Unreadable(refused) => Some(Note {
                text: format!(
                    "on disk: {} — showing the last good version",
                    refused.heading
                ),
                trouble: true,
            }),
        }
    }
}

impl Scorsese {
    /// Re-reads the open document if it changed on disk.
    ///
    /// Called at the top of every repaint, which is cheap because the watch
    /// rate-limits itself. The repaint is *asked for* because egui only draws
    /// when something happens and a file changing is not something it counts —
    /// without this an agent's edit would appear the next time the pointer
    /// moved, which is indistinguishable from not working.
    pub(super) fn follow_disk(&mut self, ctx: &egui::Context) {
        let Some(watch) = &mut self.watch else {
            return;
        };
        ctx.request_repaint_after(watch::POLL);
        if !watch.pending() {
            return;
        }
        // A hand on a clip outranks the disk. The change is deferred rather
        // than dropped — `pending` stays true — so it lands the moment the
        // gesture ends. See `watch`'s module doc for the rule.
        if self.timeline.busy() {
            return;
        }
        watch.applied();
        self.reload();
    }

    /// Re-reads the document, **leaving the viewer where it is standing**.
    ///
    /// Deliberately not [`Scorsese::pick`]'s path, which resets the playhead,
    /// the selection, the zoom and the scroll. That is right for opening a
    /// different film and wrong for the same one changing: a reload that
    /// jumped to frame zero would be barely better than relaunching the
    /// window, which is the thing this exists to stop anyone having to do.
    fn reload(&mut self) {
        let Some(current) = &self.opened else {
            return;
        };
        let root = current.root.clone();
        match open(&root) {
            Ok(fresh) => {
                // The window's own save is not news to the window. Comparing
                // documents rather than plumbing a signal out of the two
                // panels that write keeps the test for "did *someone else*
                // change this?" in one place and impossible to forget.
                if self.opened.as_ref().map(|open| &open.project) == Some(&fresh.project) {
                    self.disk = Disk::Quiet;
                    return;
                }
                self.opened = Some(fresh);
                self.disk = Disk::Reloaded;
                if let Some(open) = &self.opened {
                    self.files.refresh(open);
                }
                // The frame on screen was composited from the document that
                // has just been replaced. The playhead is untouched — where
                // someone is looking is not what changed.
                self.preview.document_changed();
            }
            // Not a crash and not an empty window. A document caught in a bad
            // state is a normal thing to see when something else is writing;
            // the last good one stays on screen and the problems are said out
            // loud, which is what the app already does for a failed open.
            Err(refused) => self.disk = Disk::Unreadable(refused),
        }
    }
}
