//! Scratch folders for tests, removed when the test that made them ends
//! (#1132).
//!
//! Not a crate: every crate's tests include this one file by path
//! (`#[path = ".../tools/scratch.rs"] mod scratch;`), because a test helper
//! is not worth a dependency edge between crates, and a copy per crate would
//! drift.
//!
//! Each test runs on a thread of its own, under libtest and nextest alike,
//! and a thread's locals are dropped when it ends — after a passing test and
//! after a panicking one. So a path handed out here is written down in a
//! thread-local list, and the list removes everything on it when it is
//! dropped. A helper can keep returning a plain `PathBuf`, and a test that
//! moves it into a struct or returns early is cleaned up all the same; a
//! guard would have to be held by every one of several hundred callers.
//!
//! Without this, `/tmp` — RAM on the maintainer's machine — gained a folder
//! per test run, every run: 5,968 of them, 631 MB, in a week. To look at what
//! a test left behind, run it with `SCORSESE_KEEP_SCRATCH=1`; the folders are
//! named on stderr instead of removed.

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

/// The paths handed out on this thread, removed when it ends.
struct Made(Vec<PathBuf>);

impl Drop for Made {
    #[expect(
        clippy::disallowed_methods,
        reason = "SCORSESE_KEEP_SCRATCH keeps a test's folders to look at, not a secret"
    )]
    fn drop(&mut self) {
        let keep = std::env::var_os("SCORSESE_KEEP_SCRATCH").is_some();
        for path in &self.0 {
            if keep {
                eprintln!("scratch kept: {}", path.display());
            } else if path.is_dir() {
                let _ = std::fs::remove_dir_all(path);
            } else {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

thread_local! {
    static MADE: RefCell<Made> = const { RefCell::new(Made(Vec::new())) };
}

/// A path in the system's temporary directory that nothing holds yet —
/// `{stem}-{process}-{n}{extension}`, unique to this call — removed with
/// everything under it when the calling thread ends. Nothing is created.
pub(crate) fn fresh(stem: &str, extension: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path =
        std::env::temp_dir().join(format!("{stem}-{}-{unique}{extension}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    MADE.with(|made| made.borrow_mut().0.push(path.clone()));
    path
}

/// A fresh, empty directory, `scorsese-{label}-…`, made now and removed with
/// everything under it when the calling thread ends: what most tests want
/// from [`fresh`], so no crate keeps its own wrapper around it.
#[allow(
    dead_code,
    reason = "each crate's tests use their own part of this file"
)]
pub(crate) fn dir(label: &str) -> PathBuf {
    let dir = fresh(&format!("scorsese-{label}"), "");
    std::fs::create_dir_all(&dir).expect("a scratch directory can be made");
    dir
}
