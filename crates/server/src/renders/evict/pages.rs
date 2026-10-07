//! Page captures under the render cache's rule (#849).
//!
//! Each project's page cache is `captures/pages/<user>/<project>/`
//! ([`crate::captures::Spool::pages`]), the `cache/` its jobs link to, and
//! `scorsese_render::page` keeps a capture in it as a **slot**,
//! `pages/<slot>/frames.mkv` beside `capture.json`. A capture is rebuildable
//! exactly as a render is, so it is held to the same rule and counted against
//! the same quota: on pressure and weekly, every slot not used in [`IDLE`]
//! goes, and the sweep also removes the folder of a project or an account that
//! no longer exists.
//!
//! - **Used** is the newest modification time of the slot's frames and
//!   record, or of any piece's (`part-<first>-<end>.mkv` and `.json`, a capture
//!   of only some of the page's frames, #809), so a slot holding only pieces
//!   ages by them and goes whole. A capture sets it; a render that reuses one
//!   sets it again ([`touch`]) — the server's job, since `scorsese-render` has
//!   no reason to know anybody ages its cache.
//! - **Held** folders are never touched at all: one a job of this process has
//!   pinned ([`super::hold`]), and one a job folder's `cache/` links to. While a
//!   job holds a project no capture of it is idle, and none half-written.
//! - **A leftover `<stem>.<pid>.partial.mkv`** (`frames.…` or a piece's
//!   `part-….`) in a folder nobody holds is a capture killed at its deadline,
//!   and goes whatever its age.
//! - A project left with no slot loses its folder, shipped fonts and all; the
//!   next capture writes them again.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use scorsese_core::CACHE_DIR;
use scorsese_render::page::{Request, cached};

use super::{Evicted, IDLE, RenderCache};

/// The folder inside a project's page cache that holds its slots, and the one
/// beside them that is not a slot — `scorsese_render::page`'s layout.
const SLOTS: &str = "pages";
const FONTS: &str = "fonts";
/// A capture's frames and record, inside a slot.
const FRAMES: &str = "frames.mkv";
const RECORD: &str = "capture.json";
/// A piece's frames and record start with this, inside a slot.
const PART: &str = "part-";

/// Mark every capture `requests` reuses as used now. `project_root` is the
/// laid-out project whose `cache/` is the page cache. A file that cannot be
/// stamped is said and left: at worst it is captured again.
///
/// Only a whole capture is stamped: the capture container takes every page
/// whole (`capture_following`), so a slot here holds no piece for a render to
/// read instead. One that somehow did would still age by its own write time.
pub fn touch(project_root: &Path, requests: &[Request], chrome_version: &str) {
    for request in requests {
        let Some(captured) = cached(project_root, request, chrome_version) else {
            continue;
        };
        let stamped = std::fs::File::options()
            .append(true)
            .open(&captured.file)
            .and_then(|file| file.set_modified(SystemTime::now()));
        if let Err(error) = stamped {
            eprintln!(
                "scorsese-server: could not mark {} as used: {error}",
                captured.file.display()
            );
        }
    }
}

/// Every project's page cache: `(user, project, folder)`, the folder
/// absolute so it compares with what [`held`] says.
pub(super) fn projects(cache: &RenderCache) -> Vec<(i64, i64, PathBuf)> {
    let mut found = Vec::new();
    for user in entries(&absolute(cache.captures().root().join(SLOTS))) {
        let Some(user_id) = number(&user) else {
            continue;
        };
        for project in entries(&user) {
            if let Some(project_id) = number(&project) {
                found.push((user_id, project_id, project));
            }
        }
    }
    found
}

/// The project folders nobody may touch now: pinned by a job of this
/// process, or linked to from a job's folder.
pub(super) fn held(cache: &RenderCache) -> HashSet<PathBuf> {
    let mut held: HashSet<PathBuf> = cache
        .pinned_paths()
        .into_iter()
        .map(|relative| absolute(cache.absolute(&relative)))
        .collect();
    for job in entries(&cache.captures().jobs()) {
        let link = job.join(crate::captures::PROJECT).join(CACHE_DIR);
        if let Ok(target) = std::fs::read_link(&link) {
            held.insert(absolute(target));
        }
    }
    held
}

/// Bytes every project's page cache holds.
pub(super) fn total(cache: &RenderCache) -> u64 {
    projects(cache)
        .iter()
        .map(|(_, _, folder)| size(folder))
        .sum()
}

/// Remove every slot idle longer than [`IDLE`] and every leftover partial in
/// `folder`, then the folder itself if no slot is left.
pub(super) fn idle(folder: &Path) -> Evicted {
    let mut evicted = Evicted::default();
    for slot in entries(&folder.join(SLOTS)) {
        if slot.file_name().is_some_and(|name| name == FONTS) {
            continue;
        }
        let used = last_used(&slot);
        for partial in entries(&slot).into_iter().filter(|file| is_partial(file)) {
            evicted.bytes += size(&partial);
            let _ = std::fs::remove_file(partial);
        }
        if used.is_none_or(|age| age >= IDLE) {
            evicted.bytes += size(&slot);
            remove(&slot);
            evicted.captures += 1;
        }
    }
    let slots_left = entries(&folder.join(SLOTS))
        .iter()
        .any(|slot| slot.file_name().is_some_and(|name| name != FONTS));
    if !slots_left {
        evicted.bytes += size(folder);
        remove(folder);
    }
    evicted
}

/// Remove a whole project folder, everything in it counted.
pub(super) fn gone(folder: &Path) -> Evicted {
    let evicted = Evicted {
        captures: entries(&folder.join(SLOTS))
            .iter()
            .filter(|slot| slot.file_name().is_some_and(|name| name != FONTS))
            .count() as u64,
        bytes: size(folder),
        ..Evicted::default()
    };
    remove(folder);
    evicted
}

/// A user's folder with no project left in it goes too. Only succeeds once
/// it is empty, which is the point.
pub(super) fn tidy(folder: &Path) {
    if let Some(user) = folder.parent() {
        let _ = std::fs::remove_dir(user);
    }
}

/// How long ago the slot's capture was last used: the newest of its frames,
/// its record and its pieces'. `None` when it has none — a capture that never
/// finished.
fn last_used(slot: &Path) -> Option<Duration> {
    entries(slot)
        .into_iter()
        .filter(|file| is_capture(file))
        .filter_map(|file| std::fs::metadata(file).ok()?.modified().ok())
        .max()
        .map(|at| SystemTime::now().duration_since(at).unwrap_or_default())
}

/// The whole capture's frames or record, or a finished piece's.
fn is_capture(file: &Path) -> bool {
    file.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name == FRAMES || name == RECORD || (name.starts_with(PART) && !is_partial(file))
        })
}

/// `<stem>.<pid>.partial.mkv`, a capture or a piece cut short.
fn is_partial(file: &Path) -> bool {
    file.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            (name.starts_with("frames.") || name.starts_with(PART))
                && name.ends_with(".partial.mkv")
        })
}

/// Bytes under `path`, links not followed.
fn size(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if meta.is_dir() {
        entries(path).iter().map(|entry| size(entry)).sum()
    } else {
        meta.len()
    }
}

fn entries(path: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(path)
        .map(|read| read.flatten().map(|entry| entry.path()).collect())
        .unwrap_or_default()
}

fn absolute(path: PathBuf) -> PathBuf {
    std::path::absolute(&path).unwrap_or(path)
}

fn number(path: &Path) -> Option<i64> {
    path.file_name()?.to_str()?.parse().ok()
}

/// Delete a folder. One already gone is deleted; anything else is said, and
/// the next pass tries again.
fn remove(path: &Path) {
    match std::fs::remove_dir_all(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            eprintln!(
                "scorsese-server: could not delete {}: {error}",
                path.display()
            );
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests;
