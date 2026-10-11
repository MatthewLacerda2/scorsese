//! What the runs in this process have been handed out of the cache, and the
//! tidy that drops what is no longer worth keeping once a run is over (#1085).
//!
//! A render captures clip after clip and decodes them all afterwards, so a
//! tidy inside each capture could delete the file an earlier clip had already
//! been handed: two clips of one page, a head-trimmed one's piece dropped
//! because the next asked for the whole page. So a capture only ever adds. A
//! [`Held`] notes every file its run is handed and every shelf it wrote, and
//! when it is dropped lets go of the files and tidies those shelves — passing
//! over every file another run in this process still holds.
//!
//! Every deletion and every hold happens under one lock, so a file is either
//! held before a tidy looks or gone before the hold is asked for: [`Held::hold`]
//! answers false for a file that went, and the caller looks again. Two
//! processes on one project (the CLI beside the MCP server) do not see each
//! other's holds.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use super::{Record, keep, pieces, shelves};
use crate::page::request::Request;

/// Every file a run in this process holds, with how many runs hold it.
static HELD: Mutex<BTreeMap<PathBuf, usize>> = Mutex::new(BTreeMap::new());

/// The holds, locked. A run that panicked holding the lock left a map that is
/// still a map.
fn held() -> MutexGuard<'static, BTreeMap<PathBuf, usize>> {
    HELD.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The files one run has been handed and the shelves it wrote, tidied and let
/// go when it is dropped.
#[derive(Debug, Default)]
pub(crate) struct Held {
    files: Vec<PathBuf>,
    written: Vec<Written>,
}

/// A shelf a run captured onto, and what to tell its fresh captures by.
#[derive(Debug)]
struct Written {
    project_root: PathBuf,
    slot: PathBuf,
    shelf: PathBuf,
    request: Request,
}

/// Where a finished capture goes: its project, slot and shelf, its frames'
/// and its record's files, and the record.
pub(crate) struct Place<'a> {
    pub(crate) project_root: &'a Path,
    pub(crate) slot: &'a Path,
    pub(crate) shelf: &'a Path,
    pub(crate) files: (&'a Path, &'a Path),
    pub(crate) record: &'a Record,
}

impl Held {
    /// Holds `file` for this run, if it is still there: false when a tidy
    /// dropped it since it was found.
    pub(crate) fn hold(&mut self, file: &Path) -> bool {
        let mut held = held();
        if !file.is_file() {
            return false;
        }
        *held.entry(file.to_owned()).or_default() += 1;
        self.files.push(file.to_owned());
        true
    }

    /// Moves the finished capture `partial` into its place, writes its record
    /// beside it — which is what makes it usable — and holds it for this run.
    pub(crate) fn place(
        &mut self,
        partial: &Path,
        place: Place<'_>,
        request: &Request,
    ) -> std::io::Result<()> {
        let mut held = held();
        let (file, record) = place.files;
        std::fs::create_dir_all(place.shelf)?;
        std::fs::rename(partial, file)?;
        keep(record, place.record)?;
        *held.entry(file.to_owned()).or_default() += 1;
        self.files.push(file.to_owned());
        if !self.written.iter().any(|w| w.shelf == place.shelf) {
            self.written.push(Written {
                project_root: place.project_root.to_owned(),
                slot: place.slot.to_owned(),
                shelf: place.shelf.to_owned(),
                request: request.clone(),
            });
        }
        Ok(())
    }
}

impl Drop for Held {
    /// The holds let go, then the tidy: the pieces each written shelf no
    /// longer needs, and the shelves past the most a slot keeps — none of them
    /// holding a file another run here still holds.
    fn drop(&mut self) {
        // Decided without the lock, since it hashes what the pages loaded;
        // only letting go and deleting wait for it, and the holds are checked
        // again there.
        let mut pieces = Vec::new();
        let mut shelves: Vec<PathBuf> = Vec::new();
        for written in &self.written {
            pieces.extend(pieces::unneeded(
                &written.shelf,
                &written.project_root,
                &written.request,
            ));
            for shelf in shelves::retired(&written.slot) {
                if !shelves.contains(&shelf) {
                    shelves.push(shelf);
                }
            }
        }
        let mut held = held();
        for file in self.files.drain(..) {
            if let Some(count) = held.get_mut(&file) {
                *count -= 1;
                if *count == 0 {
                    held.remove(&file);
                }
            }
        }
        for (file, record) in pieces {
            if !held.contains_key(&file) {
                // Record first: frames without one are never read, so a
                // removal cut short leaves nothing that looks usable.
                let _ = std::fs::remove_file(record);
                let _ = std::fs::remove_file(file);
            }
        }
        for shelf in shelves {
            if !held.keys().any(|file| file.starts_with(&shelf)) {
                let _ = std::fs::remove_dir_all(shelf);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use scorsese_compositor::Resolution;
    use scorsese_core::{Fps, hash_bytes};

    use super::super::files;
    use super::*;
    use crate::page::told::Told;

    /// A one-second page at 30 fps: 31 frames.
    fn request() -> Request {
        Request {
            page: "page.html".into(),
            resolution: Resolution::new(64, 64).unwrap(),
            fps: Fps::THIRTY,
            duration: 1.0,
            clips: BTreeMap::new(),
            words: BTreeMap::new(),
        }
    }

    /// A project of one page, and the slot its captures go in.
    fn project(name: &str) -> (PathBuf, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("scorsese-held-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("page.html"), "page").unwrap();
        (root.clone(), root.join("slot"))
    }

    /// Captures `piece` onto `shelf` of `slot` for `run`, as a capture would.
    fn capture(
        run: &mut Held,
        root: &Path,
        (slot, shelf): (&Path, &str),
        piece: std::ops::Range<u64>,
    ) -> PathBuf {
        let shelf = slot.join(shelf);
        let (file, record) = files(&shelf, &piece, &request());
        std::fs::create_dir_all(slot).unwrap();
        let partial = slot.join("partial");
        std::fs::write(&partial, "frames").unwrap();
        let loaded = BTreeMap::from([("page.html".into(), Some(hash_bytes(b"page")))]);
        let record_ = Record {
            loaded,
            warnings: Vec::new(),
            told: Told::default(),
        };
        let place = Place {
            project_root: root,
            slot,
            shelf: &shelf,
            files: (&file, &record),
            record: &record_,
        };
        run.place(&partial, place, &request()).unwrap();
        file
    }

    #[test]
    fn a_file_a_run_was_handed_outlasts_every_tidy_until_the_run_is_over() {
        let (root, slot) = project("pieces");
        let mut render = Held::default();
        let trimmed = capture(&mut render, &root, (&slot, "told-none"), 9..31);
        {
            let mut another = Held::default();
            capture(&mut another, &root, (&slot, "told-none"), 0..31);
        }
        assert!(trimmed.is_file(), "held by a run still going");
        let whole = slot.join("told-none").join(super::super::FRAMES);
        assert!(render.hold(&whole));
        drop(render);
        assert!(!trimmed.is_file(), "tidied once nobody holds it");
        assert!(whole.is_file(), "the whole page holds it");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_shelf_holding_a_held_file_is_never_retired() {
        let (root, slot) = project("shelves");
        let shelf = |run: &mut Held, name: &str| {
            std::thread::sleep(std::time::Duration::from_millis(10));
            capture(run, &root, (&slot, name), 0..31)
        };
        let mut render = Held::default();
        let shelves: Vec<PathBuf> = (0..shelves::SHELVES + 1)
            .map(|i| shelf(&mut render, &format!("told-{i:02}")))
            .collect();
        drop(render);
        assert!(!shelves[0].is_file(), "past the most a slot keeps");
        assert!(shelves[1..].iter().all(|file| file.is_file()));

        let mut render = Held::default();
        assert!(render.hold(&shelves[1]));
        shelf(&mut Held::default(), "told-last");
        assert!(shelves[1].is_file(), "the oldest, held");
        drop(render);
        shelf(&mut Held::default(), "told-later");
        assert!(!shelves[1].is_file(), "the oldest, once nobody holds it");
        assert!(!shelves[2].is_file(), "and the next, now past the most");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_file_gone_is_not_held() {
        let (root, _) = project("gone");
        let mut run = Held::default();
        assert!(!run.hold(&root.join("gone.mkv")));
        assert!(run.hold(&root.join("page.html")));
        assert_eq!(held().get(&root.join("page.html")), Some(&1));
        drop(run);
        assert_eq!(held().get(&root.join("page.html")), None);
        let _ = std::fs::remove_dir_all(&root);
    }
}
