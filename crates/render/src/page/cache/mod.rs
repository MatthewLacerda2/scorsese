//! Where captures are kept, and when one can be used again.
//!
//! A capture is deterministic and rebuildable, so it lives in `cache/` — under
//! `cache/pages/<slot>/`, as `frames.mkv` beside `capture.json`.
//!
//! **The slot is named by what was asked**: the page's path, the raster, the
//! rate, the clock's length, the browser's version, [`super::PAGE_VERSION`] and
//! the shipped fonts. **Its contents are checked against what was loaded**: the
//! page itself and every file it asked for, by hash, recorded while it ran —
//! the shipped libraries at `https://lib.scorsese/` among them, checked
//! against what this build serves. A page edited, a picture it shows
//! replaced, or a library it loads upgraded finds its slot holding a stale
//! capture, which is captured again in place — so editing a page does not grow
//! the cache by a capture per edit, and upgrading lottie-web draws again only
//! the pages that load it.
//!
//! **A slot also keeps pieces** (#809): a capture of only some of the page's
//! frames — the few around a still, or a clip's stretch from its `source_in`
//! on — as `part-<first>-<end>.mkv` beside `part-<first>-<end>.json`, each with
//! its own record, since a page that loads a file late loads it only in the
//! pieces that reach that far. The whole capture is still `frames.mkv`, which
//! is all a caller that asks for the whole page ([`super::cached`]) ever reads.
//! A piece is dropped once it goes stale or a newer capture holds all of it.
//!
//! **A slot has a shelf for each thing its page was told** (#810): the
//! captures of a page that read where some clips are, told one set of places,
//! sit in `told-<hash>/`, and one told another set in another. Which clips a
//! page reads is only known once it has run, so the slot cannot be named by
//! them; and the same page placed twice is told two sets of places in one
//! render, so neither capture may replace the other. A page that read no clip
//! has the one shelf, `told-none/`. Only the [`shelves::SHELVES`] written most
//! recently are kept, so moving a clip a page reads, again and again, does not
//! grow the cache by a capture each time.
//!
//! **A capture only ever adds; a run tidies once it is over** (#1085): which
//! pieces and shelves are no longer worth keeping is decided when the run that
//! wrote them is done with every file it was handed, never mid-render
//! ([`held`]'s).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use scorsese_core::pool::hash_file;
use scorsese_core::{CACHE_DIR, hash_bytes};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::request::Request;
use super::told::Told;

mod fontconfig;
mod held;
mod pieces;
mod shelves;

pub(crate) use fontconfig::fonts;
pub(crate) use held::{Held, Place};
pub(crate) use pieces::files;
pub(crate) use shelves::shelf;

/// The folder under `cache/` captures are kept in.
const PAGES: &str = "pages";
/// The captured frames, in a slot.
pub(crate) const FRAMES: &str = "frames.mkv";
/// What the capture loaded and warned about, in a slot.
pub(super) const RECORD: &str = "capture.json";

/// What a capture recorded about itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Record {
    /// Every project file the page loaded, by path, with its hash — `None` for
    /// one it asked for and did not find — and every shipped file, by its URL.
    pub(crate) loaded: BTreeMap<String, Option<String>>,
    /// What the capture warned about, said again whenever it is reused.
    pub(crate) warnings: Vec<String>,
    /// The places of the clips the page read, which have to be where they
    /// are now too.
    #[serde(default)]
    pub(crate) told: Told,
}

/// The folder a request's capture is kept in.
pub(crate) fn slot(project_root: &Path, request: &Request, chrome_version: &str) -> PathBuf {
    let asked = json!({
        "page": request.page,
        "width": request.resolution.width(),
        "height": request.resolution.height(),
        "fps": [request.fps.num(), request.fps.den()],
        "duration": request.duration.to_bits(),
        "chrome": chrome_version,
        "version": super::PAGE_VERSION,
        "fonts": fontconfig::fonts_hash(),
    });
    let key = hash_bytes(asked.to_string().as_bytes());
    project_root.join(CACHE_DIR).join(PAGES).join(&key[..32])
}

/// The whole page's capture for `request` on any of `slot`'s shelves, and its
/// record, if it is there and still describes the project ([`read_fresh`]).
pub(crate) fn fresh(
    slot: &Path,
    project_root: &Path,
    request: &Request,
) -> Option<(PathBuf, Record)> {
    shelves::all(slot).into_iter().find_map(|shelf| {
        let frames = shelf.join(FRAMES);
        read_fresh(&frames, &shelf.join(RECORD), project_root, request)
            .map(|record| (frames, record))
    })
}

/// The capture on any of `slot`'s shelves that holds every frame `wanted`
/// names and still describes the project: the whole page's when it is fresh,
/// and otherwise the shortest fresh piece holding them. Returns its file, the
/// page frame that file begins at, and its record.
pub(crate) fn holding(
    slot: &Path,
    project_root: &Path,
    request: &Request,
    wanted: &std::ops::Range<u64>,
) -> Option<(PathBuf, u64, Record)> {
    shelves::all(slot)
        .into_iter()
        .find_map(|shelf| pieces::holding(&shelf, project_root, request, wanted))
}

/// A capture's record, if its frames are there and it still describes the
/// project and `request`: every file loaded still hashes as it did, every file
/// missed is still missing, and every clip the page read is where it was.
pub(super) fn read_fresh(
    frames: &Path,
    record: &Path,
    project_root: &Path,
    request: &Request,
) -> Option<Record> {
    if !frames.is_file() {
        return None;
    }
    let record: Record = serde_json::from_slice(&std::fs::read(record).ok()?).ok()?;
    let unchanged = record.loaded.iter().all(|(path, hash)| {
        let now = if super::origin::record_key(path).is_some() {
            super::origin::shipped_hash(path)
        } else {
            hash_file(&project_root.join(path)).ok()
        };
        &now == hash
    });
    (unchanged && record.told.holds_for(request)).then_some(record)
}

/// Writes a capture's record, which is what makes its frames usable: written
/// last, so a capture cut short leaves one that is not [`fresh`].
pub(crate) fn keep(file: &Path, record: &Record) -> std::io::Result<()> {
    let bytes = serde_json::to_vec_pretty(record).expect("a record always serialises");
    std::fs::write(file, bytes)
}

/// A name beside `file` that no other writer is using, in this process or
/// another: `file`'s name, this process's id, a number no other call in it has
/// had, and `suffix`. What a file is written under before it is moved into
/// place, so two renders of one project never write into one file — the MCP
/// server runs its renders as threads of one process.
pub(crate) fn temporary(file: &Path, suffix: &str) -> PathBuf {
    static WRITES: AtomicU64 = AtomicU64::new(0);
    let mut name = file.file_name().unwrap_or_default().to_owned();
    name.push(format!(
        ".{}-{}.{suffix}",
        std::process::id(),
        WRITES.fetch_add(1, Ordering::Relaxed)
    ));
    file.with_file_name(name)
}

/// Writes `bytes` to `file` whole or not at all: beside it under a
/// [`temporary`] name, then moved into place, so a reader never sees it half
/// written.
pub(crate) fn write_whole(file: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let beside = temporary(file, "tmp");
    std::fs::write(&beside, bytes)
        .and_then(|()| std::fs::rename(&beside, file))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&beside);
        })
}

#[cfg(test)]
mod tests {
    use scorsese_compositor::Resolution;
    use scorsese_core::Fps;

    use super::*;
    use crate::page::told::{Read, Span};

    fn request() -> Request {
        Request {
            page: "pages/a.html".into(),
            resolution: Resolution::new(64, 64).unwrap(),
            fps: Fps::THIRTY,
            duration: 1.0,
            clips: BTreeMap::new(),
            words: BTreeMap::new(),
        }
    }

    #[test]
    fn the_slot_moves_with_everything_that_changes_the_pixels() {
        let root = Path::new("/p");
        let base = slot(root, &request(), "154");
        assert!(base.starts_with("/p/cache/pages"));
        assert_eq!(base, slot(root, &request(), "154"));
        let others = [
            slot(
                root,
                &Request {
                    page: "pages/b.html".into(),
                    ..request()
                },
                "154",
            ),
            slot(
                root,
                &Request {
                    resolution: Resolution::new(64, 32).unwrap(),
                    ..request()
                },
                "154",
            ),
            slot(
                root,
                &Request {
                    fps: Fps::PAL,
                    ..request()
                },
                "154",
            ),
            slot(
                root,
                &Request {
                    duration: 1.5,
                    ..request()
                },
                "154",
            ),
            slot(root, &request(), "155"),
        ];
        for other in others {
            assert_ne!(other, base);
        }
        let told = Request {
            clips: BTreeMap::from([(
                "vo".into(),
                Span {
                    start: 1.0,
                    end: 2.0,
                },
            )]),
            ..request()
        };
        assert_eq!(
            slot(root, &told, "154"),
            base,
            "where clips are is the shelf's to say, once a page has read them"
        );
    }

    #[test]
    fn a_capture_is_fresh_only_while_what_it_loaded_and_read_is_unchanged() {
        let root = std::env::temp_dir().join(format!("scorsese-cache-{}", std::process::id()));
        let slot = root.join("slot");
        let read = Read {
            names: ["vo".into()].into(),
            listed: false,
            ..Read::default()
        };
        let at = |start: f64| Request {
            clips: BTreeMap::from([("vo".into(), Span { start, end: 9.0 })]),
            ..request()
        };
        let shelf = shelf(&slot, &Told::of(&at(1.0), &read));
        std::fs::create_dir_all(&shelf).unwrap();
        std::fs::write(root.join("page.html"), "one").unwrap();
        std::fs::write(shelf.join(FRAMES), "frames").unwrap();
        let record = Record {
            loaded: BTreeMap::from([
                ("page.html".into(), Some(hash_bytes(b"one"))),
                ("missing.png".into(), None),
            ]),
            warnings: vec!["said".into()],
            told: Told::of(&at(1.0), &read),
        };
        let fresh = |request: &Request| fresh(&slot, &root, request).map(|(_, record)| record);
        assert_eq!(fresh(&at(1.0)), None, "no record, no capture");
        keep(&shelf.join(RECORD), &record).unwrap();
        assert_eq!(fresh(&at(1.0)), Some(record.clone()));
        assert_eq!(fresh(&at(1.5)), None, "a clip it read has moved");
        std::fs::write(root.join("missing.png"), "now here").unwrap();
        assert_eq!(
            fresh(&at(1.0)),
            None,
            "a file that appeared changes the page"
        );
        std::fs::remove_file(root.join("missing.png")).unwrap();
        std::fs::write(root.join("page.html"), "two").unwrap();
        assert_eq!(fresh(&at(1.0)), None, "an edited page is captured again");
        std::fs::write(root.join("page.html"), "one").unwrap();
        let mut shipped = record.clone();
        let lottie = "https://lib.scorsese/lottie.min.js".to_owned();
        shipped
            .loaded
            .insert(lottie.clone(), super::super::origin::shipped_hash(&lottie));
        keep(&shelf.join(RECORD), &shipped).unwrap();
        assert!(
            fresh(&at(1.0)).is_some(),
            "a library unchanged keeps it fresh"
        );
        shipped
            .loaded
            .insert(lottie, Some(hash_bytes(b"another lottie-web")));
        keep(&shelf.join(RECORD), &shipped).unwrap();
        assert_eq!(
            fresh(&at(1.0)),
            None,
            "a library upgraded draws its pages again"
        );
        let mut refused = record.clone();
        refused
            .loaded
            .insert("https://lib.scorsese/anime.min.js".into(), None);
        keep(&shelf.join(RECORD), &refused).unwrap();
        assert_eq!(
            fresh(&at(1.0)),
            None,
            "a library refused then and served now"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn no_two_writers_share_a_temporary_name_even_on_two_threads() {
        let file = Path::new("/s/part-0-30");
        let names: Vec<PathBuf> = std::thread::scope(|scope| {
            let threads: Vec<_> = (0..4)
                .map(|_| scope.spawn(|| temporary(file, "partial.mkv")))
                .collect();
            threads.into_iter().map(|t| t.join().unwrap()).collect()
        });
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), names.len(), "{names:?}");
        for name in names {
            let name = name.to_string_lossy().into_owned();
            assert!(name.starts_with("/s/part-0-30."), "{name}");
            assert!(name.ends_with(".partial.mkv"), "{name}");
        }
    }
}
