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

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use scorsese_core::pool::hash_file;
use scorsese_core::{CACHE_DIR, hash_bytes};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::request::Request;

/// The folder under `cache/` captures are kept in.
const PAGES: &str = "pages";
/// The captured frames, in a slot.
pub(crate) const FRAMES: &str = "frames.mkv";
/// What the capture loaded and warned about, in a slot.
const RECORD: &str = "capture.json";

/// What a capture recorded about itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Record {
    /// Every project file the page loaded, by path, with its hash — `None` for
    /// one it asked for and did not find.
    pub(crate) loaded: BTreeMap<String, Option<String>>,
    /// What the capture warned about, said again whenever it is reused.
    pub(crate) warnings: Vec<String>,
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

/// The record of a slot's capture, if it is there and still describes the
/// project: every file loaded still hashes as it did, and every file missed is
/// still missing.
pub(crate) fn fresh(slot: &Path, project_root: &Path) -> Option<Record> {
    if !slot.join(FRAMES).is_file() {
        return None;
    }
    let record: Record = serde_json::from_slice(&std::fs::read(slot.join(RECORD)).ok()?).ok()?;
    let unchanged = record.loaded.iter().all(|(path, hash)| {
        let now = hash_file(&project_root.join(path)).ok();
        &now == hash
    });
    unchanged.then_some(record)
}

/// Writes a slot's record, which is what makes its frames usable: written
/// last, so a capture cut short leaves a slot that is not [`fresh`].
pub(crate) fn keep(slot: &Path, record: &Record) -> std::io::Result<()> {
    let bytes = serde_json::to_vec_pretty(record).expect("a record always serialises");
    std::fs::write(slot.join(RECORD), bytes)
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

    fn request() -> Request {
        Request {
            page: "pages/a.html".into(),
            resolution: Resolution::new(64, 64).unwrap(),
            fps: Fps::THIRTY,
            duration: 1.0,
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
    }

    #[test]
    fn a_capture_is_fresh_only_while_what_it_loaded_is_unchanged() {
        let root = std::env::temp_dir().join(format!("scorsese-cache-{}", std::process::id()));
        let slot = root.join("slot");
        std::fs::create_dir_all(&slot).unwrap();
        std::fs::write(root.join("page.html"), "one").unwrap();
        std::fs::write(slot.join(FRAMES), "frames").unwrap();
        let record = Record {
            loaded: BTreeMap::from([
                ("page.html".into(), Some(hash_bytes(b"one"))),
                ("missing.png".into(), None),
            ]),
            warnings: vec!["said".into()],
        };
        assert_eq!(fresh(&slot, &root), None, "no record, no capture");
        keep(&slot, &record).unwrap();
        assert_eq!(fresh(&slot, &root), Some(record));
        std::fs::write(root.join("missing.png"), "now here").unwrap();
        assert_eq!(
            fresh(&slot, &root),
            None,
            "a file that appeared changes the page"
        );
        std::fs::remove_file(root.join("missing.png")).unwrap();
        std::fs::write(root.join("page.html"), "two").unwrap();
        assert_eq!(
            fresh(&slot, &root),
            None,
            "an edited page is captured again"
        );
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
