//! Noticing that the open project changed underneath the window.
//!
//! This is the workflow scorsese is built around: an assistant does the
//! structural editing over MCP while a person watches and reacts. Without it
//! the window reads `project.json` once at startup and never looks again, so
//! every edit an agent makes is invisible until the window is killed and
//! relaunched — which throws away the playhead, the selection, and whatever
//! was being looked at, the exact context that makes a review worth anything.
//!
//! **Polled, not watched.** Every platform has a native facility and one crate
//! covers all three, but that crate is a dependency and a new licence on the
//! allow-list for a job one `stat` does. Asking the filesystem for a single
//! file's size and modification time a few times a second costs nothing
//! measurable, and the debounce a native watcher needs — an editor or a tool
//! touching a file several times in a moment — falls out of the interval for
//! free.
//!
//! ## The write-conflict rule
//!
//! Two writers share one document: the window, when a hand comes off a clip,
//! and whoever else has the project open. The rule is **last writer wins on
//! the file**, with one deliberate exception: a change noticed while a gesture
//! is in flight is *deferred*, not applied, until the hand comes off. Yanking
//! a clip out from under a pointer mid-drag is the one case where the newer
//! document is not the one anybody wants. Nothing is merged and nothing is
//! queued beyond that — this is a document being re-read, not a collaborative
//! editor.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use scorsese_core::PROJECT_FILE_NAME;

/// How often the document is looked at. Fast enough that a person watching an
/// agent work sees the edit land, slow enough that looking costs nothing.
pub(crate) const POLL: Duration = Duration::from_millis(300);

/// What the file looked like when it was last read. The outer `None` is a file
/// that could not be stat'd at all — usually not there — which is worth
/// telling apart from one that is.
type Stamp = Option<(Option<SystemTime>, u64)>;

/// Watches one project's `project.json`.
pub(crate) struct Watch {
    /// The document being watched.
    file: PathBuf,
    /// How it looked when it was last read.
    seen: Stamp,
    /// When it was last looked at, so that looking is rate-limited.
    looked: Instant,
    /// A change that has been noticed and not yet acted on. Held rather than
    /// reported once, so a change that arrives mid-gesture survives until the
    /// gesture ends instead of being dropped.
    pending: bool,
}

impl Watch {
    /// Starts watching the project in `root`, taking the document as it stands
    /// to be the one the window is already showing.
    pub(crate) fn on(root: &Path) -> Self {
        let file = root.join(PROJECT_FILE_NAME);
        let seen = stamp(&file);
        Self {
            file,
            seen,
            looked: Instant::now(),
            pending: false,
        }
    }

    /// Whether the document is waiting to be re-read.
    ///
    /// Rate-limited rather than run on every repaint: egui repaints for
    /// reasons that have nothing to do with the file — a pointer moving, a
    /// tooltip fading — and a `stat` per repaint would be a syscall per mouse
    /// move. Stays true once set, until [`Watch::applied`] clears it.
    pub(crate) fn pending(&mut self) -> bool {
        if self.looked.elapsed() >= POLL {
            self.looked = Instant::now();
            let now = stamp(&self.file);
            if now != self.seen {
                self.seen = now;
                self.pending = true;
            }
        }
        self.pending
    }

    /// Says the pending change has been dealt with.
    pub(crate) fn applied(&mut self) {
        self.pending = false;
    }
}

/// Size and modification time, which together are what a change looks like
/// from outside.
///
/// A save is a rename over the target rather than an edit in place, so what
/// the window sees next is a different file — but neither field is guaranteed
/// to differ on its own (a coarse clock, or a document that changed without
/// changing length), and both matching by accident is not something an edit
/// does.
fn stamp(file: &Path) -> Stamp {
    let data = std::fs::metadata(file).ok()?;
    Some((data.modified().ok(), data.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A project directory with a document in it, removed when the test ends.
    struct Fixture(PathBuf);

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn project(label: &str) -> Fixture {
        let dir = std::env::temp_dir().join(format!("scorsese-watch-{label}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create the project directory");
        std::fs::write(dir.join(PROJECT_FILE_NAME), "{}").expect("write a document");
        Fixture(dir)
    }

    /// A window sitting still must not keep announcing that nothing happened.
    #[test]
    fn an_untouched_document_is_never_a_change() {
        let project = project("quiet");
        let mut watch = Watch::on(&project.0);

        std::thread::sleep(POLL * 2);

        assert!(!watch.pending());
    }

    #[test]
    fn a_document_written_by_something_else_is_noticed() {
        let project = project("noticed");
        let mut watch = Watch::on(&project.0);

        std::fs::write(project.0.join(PROJECT_FILE_NAME), "{ \"changed\": true }")
            .expect("something else writes");
        std::thread::sleep(POLL * 2);

        assert!(watch.pending());
    }

    /// The latch. A change noticed while a hand is on a clip is deferred, not
    /// dropped — so it has to survive being asked about repeatedly and only
    /// clear when someone says they have dealt with it.
    #[test]
    fn a_noticed_change_waits_until_it_is_applied() {
        let project = project("latched");
        let mut watch = Watch::on(&project.0);
        std::fs::write(project.0.join(PROJECT_FILE_NAME), "{ \"changed\": true }")
            .expect("something else writes");
        std::thread::sleep(POLL * 2);

        assert!(watch.pending(), "the change is there to begin with");
        assert!(watch.pending(), "and asking again does not consume it");

        watch.applied();

        assert!(!watch.pending(), "once dealt with it is gone");
    }
}
