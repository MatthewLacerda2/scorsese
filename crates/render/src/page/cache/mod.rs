//! Where captures are kept, and when one can be used again.
//!
//! A capture is deterministic and rebuildable, so it lives in `cache/` — under
//! `cache/pages/<slot>/`, as `frames.mkv` beside `capture.json`.
//!
//! **The slot is named by what was asked**: the page's path, the raster, the
//! rate, the clock's length, the browser's version, [`super::PAGE_VERSION`] and
//! the shipped fonts. **Its contents are checked against what was loaded**: the
//! page itself and every file it asked for, by hash, recorded while it ran. A
//! page edited, or a picture it shows replaced, finds its slot holding a stale
//! capture, which is captured again in place — so editing a page does not grow
//! the cache by a capture per edit.
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
//! has the one shelf, `told-none/`. Only the [`SHELVES`] written most recently
//! are kept, so moving a clip a page reads, again and again, does not grow the
//! cache by a capture each time.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use scorsese_core::pool::hash_file;
use scorsese_core::{CACHE_DIR, hash_bytes};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::request::Request;
use super::told::Told;

mod pieces;

pub(crate) use pieces::{files, prune};

/// The folder under `cache/` captures are kept in.
const PAGES: &str = "pages";
/// The captured frames, in a slot.
pub(crate) const FRAMES: &str = "frames.mkv";
/// What the capture loaded and warned about, in a slot.
pub(super) const RECORD: &str = "capture.json";
/// How many shelves a slot keeps: a page placed this many times in one edit,
/// each told different places, is the most that is never captured twice.
const SHELVES: usize = 8;

/// What a capture recorded about itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Record {
    /// Every project file the page loaded, by path, with its hash — `None` for
    /// one it asked for and did not find.
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
        "fonts": fonts_hash(),
    });
    let key = hash_bytes(asked.to_string().as_bytes());
    project_root.join(CACHE_DIR).join(PAGES).join(&key[..32])
}

/// The shelf in `slot` for captures told `told`.
pub(crate) fn shelf(slot: &Path, told: &Told) -> PathBuf {
    slot.join(told.folder())
}

/// The whole page's capture for `request` on any of `slot`'s shelves, and its
/// record, if it is there and still describes the project ([`read_fresh`]).
pub(crate) fn fresh(
    slot: &Path,
    project_root: &Path,
    request: &Request,
) -> Option<(PathBuf, Record)> {
    shelves(slot).into_iter().find_map(|shelf| {
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
    shelves(slot)
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
        let now = hash_file(&project_root.join(path)).ok();
        &now == hash
    });
    (unchanged && record.told.holds_for(request)).then_some(record)
}

/// Drops every shelf of `slot` but the [`SHELVES`] written most recently,
/// `kept` always among them.
pub(crate) fn retire(slot: &Path, kept: &Path) {
    let mut others: Vec<(std::time::SystemTime, PathBuf)> = shelves(slot)
        .into_iter()
        .filter(|shelf| shelf != kept)
        .map(|shelf| {
            let written = std::fs::metadata(&shelf).and_then(|m| m.modified());
            (written.unwrap_or(std::time::UNIX_EPOCH), shelf)
        })
        .collect();
    others.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, shelf) in others.into_iter().skip(SHELVES - 1) {
        let _ = std::fs::remove_dir_all(shelf);
    }
}

/// Every shelf in `slot`.
fn shelves(slot: &Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(slot) else {
        return Vec::new();
    };
    let mut shelves: Vec<PathBuf> = read
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("told-"))
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    shelves.sort();
    shelves
}

/// Writes a capture's record, which is what makes its frames usable: written
/// last, so a capture cut short leaves one that is not [`fresh`].
pub(crate) fn keep(file: &Path, record: &Record) -> std::io::Result<()> {
    let bytes = serde_json::to_vec_pretty(record).expect("a record always serialises");
    std::fs::write(file, bytes)
}

/// The shipped faces ([`super::fonts`]) written out where the browser can find them, with a
/// fontconfig file listing **only** them — so a page that names a font it was
/// not given falls back to the same face on every machine, never to whatever
/// the host has installed (#606). Returns that file. Written once per project;
/// a face whose file is already there at the right length is left alone.
pub(crate) fn fonts(project_root: &Path) -> std::io::Result<PathBuf> {
    let folder = project_root.join(CACHE_DIR).join(PAGES).join("fonts");
    std::fs::create_dir_all(&folder)?;
    for face in super::fonts::faces() {
        let file = folder.join(&face.file);
        let current = std::fs::metadata(&file).map(|m| m.len()).ok();
        if current != Some(face.bytes.len() as u64) {
            std::fs::write(&file, face.bytes)?;
        }
    }
    let conf = folder.join("fonts.conf");
    let text = format!(
        "<?xml version=\"1.0\"?>\n<fontconfig>\n  <dir>{}</dir>\n  <cachedir>{}</cachedir>\n</fontconfig>\n",
        folder.display(),
        folder.join(".cache").display()
    );
    std::fs::write(&conf, text)?;
    Ok(conf)
}

/// One hash over every shipped face: a build that ships different fonts draws
/// different pages.
fn fonts_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| {
        let digests: Vec<String> = super::fonts::faces()
            .iter()
            .map(|face| format!("{}:{}", face.file, hash_bytes(face.bytes)))
            .collect();
        hash_bytes(digests.join("\n").as_bytes())
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
        assert_eq!(fresh(&at(1.0)), Some(record));
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
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_slot_keeps_only_the_shelves_written_last() {
        let root = std::env::temp_dir().join(format!("scorsese-shelves-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for i in 0..SHELVES + 2 {
            std::fs::create_dir_all(root.join(format!("told-{i:02}"))).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        std::fs::create_dir_all(root.join("fonts")).unwrap();
        let kept = root.join("told-00");
        retire(&root, &kept);
        let left = shelves(&root);
        assert_eq!(left.len(), SHELVES);
        assert!(left.contains(&kept), "the shelf just written stays");
        assert!(!left.contains(&root.join("told-01")), "the oldest go");
        assert!(root.join("fonts").is_dir(), "only shelves are retired");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_browser_gets_every_shipped_face_and_a_fontconfig_naming_only_them() {
        let root = std::env::temp_dir().join(format!("scorsese-fonts-{}", std::process::id()));
        let conf = fonts(&root).unwrap();
        let folder = conf.parent().unwrap().to_path_buf();
        assert_eq!(folder, root.join("cache/pages/fonts"));
        let text = std::fs::read_to_string(&conf).unwrap();
        assert!(
            text.contains(&format!("<dir>{}</dir>", folder.display())),
            "{text}"
        );
        let inter = folder.join("inter.ttf");
        assert_eq!(
            std::fs::read(&inter).unwrap(),
            super::super::fonts::bytes("inter.ttf").unwrap()
        );
        // A face cut short — an interrupted write — is written again.
        std::fs::write(&inter, b"short").unwrap();
        fonts(&root).unwrap();
        assert_eq!(
            std::fs::metadata(&inter).unwrap().len(),
            super::super::fonts::bytes("inter.ttf").unwrap().len() as u64
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_fonts_are_named_by_a_hash_of_their_bytes() {
        let hash = fonts_hash();
        assert_eq!(hash.len(), 64);
        assert!(hash.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_ne!(hash, hash_bytes(b""));
    }
}
