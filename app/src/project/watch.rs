//! Noticing that the open project changed underneath the window.
//!
//! This is the workflow scorsese is built around: an assistant does the
//! structural editing over MCP while a person watches and reacts. Without it
//! the window reads `project.json` once at startup and never looks again, so
//! every edit an agent makes is invisible until the window is killed and
//! relaunched — which throws away the playhead, the selection, and whatever
//! was being looked at, the exact context that makes a review worth anything.
//!
//! ## Two things are watched, for one reason
//!
//! The document is one. The other is `cache/voices/`, which `scorsese voices`
//! fills in from a terminal beside the window — and until it does, the voice
//! picker has nothing to offer and says so. That empty list is the **first**
//! state anybody meets, because scorsese ships no default voice on purpose, so
//! the one moment a person most needs the window to look again is the moment
//! they are least likely to know that they should ask it to. Same poll, same
//! interval, one more `stat`.
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
use scorsese_providers::voices;

/// How often the document is looked at. Fast enough that a person watching an
/// agent work sees the edit land, slow enough that looking costs nothing.
pub(crate) const POLL: Duration = Duration::from_millis(300);

/// What a path looked like when it was last read. The outer `None` is a path
/// that could not be stat'd at all — usually not there — which is worth
/// telling apart from one that is, and for `cache/voices/` it is the ordinary
/// state of a project nobody has fetched a list for yet.
type Stamp = Option<(Option<SystemTime>, u64)>;

/// One path being followed, and whether the last look found it different.
struct Tracked {
    /// What is being looked at.
    path: PathBuf,
    /// How it looked when it was last read.
    seen: Stamp,
    /// A change that has been noticed and not yet acted on.
    pending: bool,
}

impl Tracked {
    /// Starts following `path`, taking it as it stands to be what the window
    /// is already showing.
    fn on(path: PathBuf) -> Self {
        let seen = stamp(&path);
        Self {
            path,
            seen,
            pending: false,
        }
    }

    /// Looks once, and latches a difference.
    fn look(&mut self) {
        let now = stamp(&self.path);
        if now != self.seen {
            self.seen = now;
            self.pending = true;
        }
    }
}

/// Watches one project's `project.json`, and the voice listings cached beside
/// it.
pub(crate) struct Watch {
    /// The document.
    document: Tracked,
    /// The directory of cached voice listings, which something outside the
    /// window is what fills in.
    voices: Tracked,
    /// When they were last looked at, so that looking is rate-limited. One
    /// clock for both: a look is a pair of `stat`s, and staggering them would
    /// buy nothing and mean two intervals to reason about.
    looked: Instant,
}

impl Watch {
    /// Starts watching the project in `root`.
    pub(crate) fn on(root: &Path) -> Self {
        Self {
            document: Tracked::on(root.join(PROJECT_FILE_NAME)),
            voices: Tracked::on(voices::cache_dir(root)),
            looked: Instant::now(),
        }
    }

    /// Whether the document is waiting to be re-read.
    ///
    /// Stays true once set, until [`Watch::applied`] clears it — a change
    /// noticed while a hand is on a clip is deferred rather than dropped.
    pub(crate) fn pending(&mut self) -> bool {
        self.look();
        self.document.pending
    }

    /// Says the pending change has been dealt with.
    pub(crate) fn applied(&mut self) {
        self.document.pending = false;
    }

    /// Whether the cached voice listings changed since this was last asked.
    ///
    /// Answered once and cleared, unlike the document's — there is no gesture
    /// to defer it for. Re-reading a listing replaces nothing anybody is
    /// holding: it is a vendor's list, not the film.
    pub(crate) fn voices_changed(&mut self) -> bool {
        self.look();
        std::mem::take(&mut self.voices.pending)
    }

    /// Looks at both, at most once per [`POLL`].
    ///
    /// Rate-limited rather than run on every repaint: egui repaints for
    /// reasons that have nothing to do with the filesystem — a pointer moving,
    /// a tooltip fading — and a `stat` per repaint would be a syscall per mouse
    /// move.
    fn look(&mut self) {
        if self.looked.elapsed() < POLL {
            return;
        }
        self.looked = Instant::now();
        self.document.look();
        self.voices.look();
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
///
/// A directory is stamped by what is *in* it, and for the same reason: a
/// listing is rewritten in place, and on Linux that leaves the directory's own
/// modification time free to say nothing happened.
fn stamp(path: &Path) -> Stamp {
    let data = std::fs::metadata(path).ok()?;
    if data.is_dir() {
        return Some(inside(path));
    }
    Some((data.modified().ok(), data.len()))
}

/// A directory as one stamp: the newest modification time in it, and the total
/// number of bytes it holds.
///
/// Entries are not listed, so a file renamed to another of exactly its size in
/// the same second reads as unchanged. That is a directory the vendor's
/// listings are written into by one writer, and the escape hatch for a missed
/// look is the button that has always been there.
fn inside(dir: &Path) -> (Option<SystemTime>, u64) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (None, 0);
    };
    let mut newest = None;
    let mut bytes = 0;
    for data in entries.flatten().filter_map(|entry| entry.metadata().ok()) {
        newest = newest.max(data.modified().ok());
        bytes += data.len();
    }
    (newest, bytes)
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

    /// A voice listing landing in the project's cache, as `scorsese voices`
    /// puts one there.
    fn listing(root: &Path, body: &str) {
        let dir = voices::cache_dir(root);
        std::fs::create_dir_all(&dir).expect("create cache/voices/");
        std::fs::write(dir.join("a.json"), body).expect("write a listing");
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

    /// The state everybody meets first: no list at all, and a terminal beside
    /// the window that has just fetched one.
    #[test]
    fn the_first_listing_a_project_ever_gets_is_noticed() {
        let project = project("first-listing");
        let mut watch = Watch::on(&project.0);

        listing(&project.0, "[]");
        std::thread::sleep(POLL * 2);

        assert!(watch.voices_changed());
    }

    /// `scorsese voices --refresh` writes over the listing that is already
    /// there, which leaves the directory's own modification time free to say
    /// nothing happened. What is *in* it is what is watched.
    #[test]
    fn a_listing_written_over_an_existing_one_is_noticed() {
        let project = project("refreshed-listing");
        listing(&project.0, "[]");
        let mut watch = Watch::on(&project.0);

        listing(&project.0, "[ \"a voice\" ]");
        std::thread::sleep(POLL * 2);

        assert!(watch.voices_changed());
    }

    /// Answered once and cleared: re-reading a listing every repaint is what
    /// the picker keeps a copy to avoid.
    #[test]
    fn a_cache_nobody_has_touched_is_never_a_change() {
        let project = project("quiet-listing");
        listing(&project.0, "[]");
        let mut watch = Watch::on(&project.0);
        listing(&project.0, "[ \"a voice\" ]");
        std::thread::sleep(POLL * 2);

        assert!(watch.voices_changed(), "the change is there to begin with");
        assert!(!watch.voices_changed(), "and asking again does not find it");
    }

    /// The two are answered separately. A listing arriving is not the document
    /// changing, and re-loading the project on account of one would throw the
    /// preview away for a dropdown.
    #[test]
    fn a_listing_is_not_a_change_to_the_document() {
        let project = project("listing-not-document");
        let mut watch = Watch::on(&project.0);

        listing(&project.0, "[]");
        std::thread::sleep(POLL * 2);

        assert!(!watch.pending(), "the document did not change");
        assert!(watch.voices_changed());
    }
}
