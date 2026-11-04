//! What a capture wrote is not trusted to stay inside the page cache (#1643).
//!
//! On the web a capture container writes its project's page cache, so a link
//! or a record there may lead anywhere. Whatever reads the cache reads it as a
//! miss rather than follow one out.
//!
//! `cache/pages/` itself is the caller's to read through: whoever lays the
//! project out may link it, or `cache/`, wherever the project's captures are
//! kept, and keeps a capture from replacing it. Everything below it is the
//! capture's.
//!
//! What writes the cache holds to a stricter line ([`made`], #1698, #1718):
//! from the project root down, nothing is a link — `cache/` itself included —
//! unless the caller laid `cache/` out as one itself and says so
//! ([`Laid::cache`]).

use std::path::{Component, Path, PathBuf};

use scorsese_core::CACHE_DIR;
use scorsese_core::write::{folder_below, in_cache};

use super::PAGES;

/// How the caller laid out the project a capture is made in: what outside it
/// the capture may reach on the caller's word. Nothing, by default — a local
/// project's links lead nowhere outside it, and its `cache/` is its own.
#[derive(Debug, Clone, Copy, Default)]
pub struct Laid<'a> {
    /// Folders outside the project its links may really lead under: the
    /// owner's library, which the web links each media file from (#857). A
    /// link leading anywhere else is refused.
    pub follow: &'a [PathBuf],
    /// Whether the project's `cache/` is a link the caller made itself (the
    /// web's capture worker links it to where the project's captures are
    /// kept), and so is written through. Otherwise a linked `cache/` is
    /// refused before anything is written where it leads (#1718): a `.scor`
    /// received from someone else may carry one.
    pub cache: bool,
}

/// Makes `folder` — a slot, a shelf, the fonts — below `project_root`'s
/// `cache/pages/`, with every folder on the way, none of them a link from the
/// project root down ([`in_cache`]): what a capture asks before it writes
/// there, so a link a `.scor` received from someone else carries never leads
/// its frames out of the project. When the caller laid `cache/` out as a link
/// itself (`laid_cache`, [`Laid::cache`]), the line starts below it instead.
///
/// # Errors
///
/// If `folder` is not below `cache/pages/`, a folder on the way is a link or
/// a file, or one cannot be made.
pub(crate) fn made(project_root: &Path, laid_cache: bool, folder: &Path) -> std::io::Result<()> {
    let cache = project_root.join(CACHE_DIR);
    let below = folder.strip_prefix(cache.join(PAGES)).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{} is not in the page cache", folder.display()),
        )
    })?;
    let pages = Path::new(PAGES).join(below);
    if laid_cache {
        folder_below(&cache, &pages).map(drop)
    } else {
        in_cache(project_root, &pages).map(drop)
    }
}

/// Whether `path` is a file itself, not a link to one.
pub(super) fn is_plain_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file())
}

/// Whether every folder and file from `base` down to `path` is there and none
/// is a link; `base` itself may be one.
pub(super) fn unlinked(base: &Path, path: &Path) -> bool {
    let Ok(below) = path.strip_prefix(base) else {
        return false;
    };
    let mut at = base.to_path_buf();
    below.components().all(|part| {
        at.push(part);
        matches!(part, Component::Normal(_))
            && std::fs::symlink_metadata(&at).is_ok_and(|meta| !meta.file_type().is_symlink())
    })
}

/// Whether a record's `path` names a file a page could have been served: one
/// under the project root that neither climbs out of it nor lies in its
/// cache, which only captures write.
pub(super) fn in_project(path: &str) -> bool {
    let path = Path::new(path);
    !path.starts_with(CACHE_DIR)
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use scorsese_compositor::Resolution;
    use scorsese_core::{CACHE_DIR, Fps, hash_bytes};

    use super::super::{FRAMES, PAGES, RECORD, Record, holding, keep, read_fresh, slot};
    use crate::page::request::Request;
    use crate::page::told::Told;

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

    /// A project at `<scratch>/project` whose page reads "one", with a file
    /// outside it at `<scratch>/outside/secret`; the project and the outside.
    fn beside(name: &str) -> (PathBuf, PathBuf) {
        let scratch = crate::scratch::fresh(&format!("scorsese-cache-{name}"), "");
        let _ = std::fs::remove_dir_all(&scratch);
        let (root, outside) = (scratch.join("project"), scratch.join("outside"));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(root.join("page.html"), "one").unwrap();
        std::fs::write(outside.join("secret"), "secret").unwrap();
        (root, outside)
    }

    /// A whole capture of `request()` in `shelf` that loaded the page, and
    /// whatever else `loaded` names.
    fn captured(shelf: &Path, loaded: &[(&str, &[u8])]) {
        std::fs::create_dir_all(shelf).unwrap();
        std::fs::write(shelf.join(FRAMES), "frames").unwrap();
        let mut record = Record {
            loaded: BTreeMap::from([("page.html".into(), Some(hash_bytes(b"one")))]),
            warnings: Vec::new(),
            told: Told::default(),
        };
        for (path, bytes) in loaded {
            record
                .loaded
                .insert((*path).to_owned(), Some(hash_bytes(bytes)));
        }
        keep(&shelf.join(RECORD), &record).unwrap();
    }

    #[test]
    fn a_record_naming_a_file_outside_the_project_is_a_miss() {
        let (root, outside) = beside("climbs");
        let shelf = root.join("slot/told-none");
        let fresh = || read_fresh(&shelf.join(FRAMES), &shelf.join(RECORD), &root, &request());
        captured(&shelf, &[]);
        assert!(fresh().is_some(), "the capture itself is fine");
        for named in [
            "../outside/secret".to_owned(),
            outside.join("secret").display().to_string(),
        ] {
            captured(&shelf, &[(&named, b"secret")]);
            assert_eq!(fresh(), None, "{named} is outside the project");
        }
        std::fs::create_dir_all(root.join(CACHE_DIR)).unwrap();
        std::os::unix::fs::symlink(outside.join("secret"), root.join("cache/secret")).unwrap();
        captured(&shelf, &[("cache/secret", b"secret")]);
        assert_eq!(fresh(), None, "the cache is the capture's to write");
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    #[test]
    fn frames_or_a_record_that_is_a_link_is_a_miss() {
        let (root, outside) = beside("linked-files");
        let shelf = root.join("slot/told-none");
        captured(&outside.join("real"), &[]);
        std::fs::create_dir_all(&shelf).unwrap();
        let fresh = || read_fresh(&shelf.join(FRAMES), &shelf.join(RECORD), &root, &request());
        std::os::unix::fs::symlink(outside.join("real").join(FRAMES), shelf.join(FRAMES)).unwrap();
        std::fs::copy(outside.join("real").join(RECORD), shelf.join(RECORD)).unwrap();
        assert_eq!(fresh(), None, "frames that are a link");
        std::fs::remove_file(shelf.join(FRAMES)).unwrap();
        std::fs::copy(outside.join("real").join(FRAMES), shelf.join(FRAMES)).unwrap();
        assert!(fresh().is_some());
        std::fs::remove_file(shelf.join(RECORD)).unwrap();
        std::os::unix::fs::symlink(outside.join("real").join(RECORD), shelf.join(RECORD)).unwrap();
        assert_eq!(fresh(), None, "a record that is a link");
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    #[test]
    fn a_slot_or_shelf_that_is_a_link_holds_nothing() {
        let (root, outside) = beside("linked-folders");
        let slot = slot(&root, &request(), "154");
        let key = slot.file_name().unwrap().to_owned();
        let pages = root.join(CACHE_DIR).join(PAGES);
        let find = || holding(&slot, &root, &request(), &(0..31)).map(|(file, ..)| file);
        let link = |from: &Path, to: &Path| {
            let _ = std::fs::remove_dir_all(from);
            let _ = std::fs::remove_file(from);
            std::fs::create_dir_all(from.parent().unwrap()).unwrap();
            std::os::unix::fs::symlink(to, from).unwrap();
        };
        captured(&slot.join("told-none"), &[]);
        assert!(find().is_some(), "a capture where it belongs is found");

        captured(&outside.join("shelf"), &[]);
        link(&slot.join("told-none"), &outside.join("shelf"));
        assert_eq!(find(), None, "a shelf that is a link");

        captured(&outside.join("slot/told-none"), &[]);
        link(&slot, &outside.join("slot"));
        assert_eq!(find(), None, "a slot that is a link");

        // `pages/` is the caller's: a web still links it to the project's kept
        // captures, and keeps a capture from replacing it.
        let kept = outside.join("kept");
        captured(&kept.join(&key).join("told-none"), &[]);
        link(&pages, &kept);
        assert!(find().is_some(), "a pages folder the caller linked");
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    /// A slot, a shelf or the fonts folder that is a link is refused before
    /// anything is made where it leads (#1698); `cache/pages/` itself is the
    /// caller's to link.
    #[test]
    fn a_folder_below_the_page_cache_that_is_a_link_is_never_made_through() {
        let (root, outside) = beside("made-linked");
        let pages = root.join(CACHE_DIR).join(PAGES);
        let slot = slot(&root, &request(), "154");
        std::fs::create_dir_all(&pages).unwrap();
        std::os::unix::fs::symlink(&outside, &slot).unwrap();
        assert!(super::made(&root, false, &slot.join("told-none")).is_err());
        std::fs::remove_file(&slot).unwrap();
        std::fs::create_dir(&slot).unwrap();
        std::os::unix::fs::symlink(&outside, slot.join("told-none")).unwrap();
        assert!(super::made(&root, false, &slot.join("told-none")).is_err());
        std::os::unix::fs::symlink(&outside, pages.join("fonts")).unwrap();
        assert!(super::super::fonts(&root, false).is_err());
        std::fs::remove_file(pages.join("fonts")).unwrap();
        std::fs::create_dir(pages.join("fonts")).unwrap();
        std::os::unix::fs::symlink(&outside, pages.join("fonts/.cache")).unwrap();
        assert!(
            super::super::fonts(&root, false).is_err(),
            "fontconfig's own cache"
        );
        let landed: Vec<_> = std::fs::read_dir(&outside).unwrap().collect();
        assert_eq!(landed.len(), 1, "only the secret: {landed:?}");
        assert!(super::made(&root, false, &root.join("elsewhere")).is_err());
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    /// Locally the line starts at the project root (#1718): `cache/` or
    /// `cache/pages/` a link is refused before anything is made where it
    /// leads.
    #[test]
    fn a_linked_cache_or_page_cache_is_refused_locally() {
        let (root, outside) = beside("made-base-linked");
        let slot = slot(&root, &request(), "154");
        std::os::unix::fs::symlink(&outside, root.join(CACHE_DIR)).unwrap();
        assert!(super::made(&root, false, &slot.join("told-none")).is_err());
        std::fs::remove_file(root.join(CACHE_DIR)).unwrap();
        std::fs::create_dir(root.join(CACHE_DIR)).unwrap();
        std::os::unix::fs::symlink(&outside, root.join(CACHE_DIR).join(PAGES)).unwrap();
        assert!(super::made(&root, false, &slot.join("told-none")).is_err());
        let landed: Vec<_> = std::fs::read_dir(&outside).unwrap().collect();
        assert_eq!(landed.len(), 1, "only the secret: {landed:?}");
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    /// A `cache/` the caller laid out as a link itself (the web's capture
    /// worker) is written through, and the line starts below it.
    #[test]
    fn a_cache_the_caller_linked_is_written_through() {
        let (root, outside) = beside("made-laid");
        std::os::unix::fs::symlink(&outside, root.join(CACHE_DIR)).unwrap();
        let slot = slot(&root, &request(), "154");
        super::made(&root, true, &slot.join("told-none")).expect("the caller's link");
        let key = slot.file_name().unwrap();
        assert!(outside.join(PAGES).join(key).join("told-none").is_dir());
        std::fs::remove_dir_all(outside.join(PAGES).join(key)).unwrap();
        std::os::unix::fs::symlink(root.parent().unwrap(), outside.join(PAGES).join(key)).unwrap();
        assert!(super::made(&root, true, &slot.join("told-none")).is_err());
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }
}
