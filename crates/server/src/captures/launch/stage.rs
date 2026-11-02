//! Copying a job into staging, and keeping what a capture wrote (#1636,
//! #1677): every entry of the server's folders read or written from a handle
//! ([`super::handle`]), so what is checked is what is copied.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use scorsese_core::CACHE_DIR;

use super::handle::{Folder, Kind};
use super::{Launch, OUT, STAGED_JOBS, cache_parts, is_number, is_plain, library_of, plain};
use crate::captures::{ASK, Ask, JOBS, PAGES, PROJECT};

/// A job copied into staging ([`Launch::stage`]): what its captures mount.
#[derive(Debug)]
pub struct Staged {
    /// The job folder's name, `job-<n>` or `still-<call>`.
    pub(super) name: String,
    /// Whose page cache it captures into: the user's and the project's ids.
    pub(super) cache: (String, String),
    /// The owner's media its project links to, at the paths the links name.
    pub(super) media: BTreeSet<PathBuf>,
}

impl Launch {
    /// Copies the job at `job` into staging, asking `ask`, or says why it is
    /// not a job a capture may be given: every entry read from a handle
    /// (`launch::handle`), so what is checked is what is copied.
    pub fn stage(&self, job: &Path, ask: &Ask) -> Result<Staged, String> {
        let name = job
            .file_name()
            .and_then(OsStr::to_str)
            .filter(|name| plain(Path::new(name)).is_some())
            .ok_or("the job's folder has a name a mount cannot carry")?
            .to_owned();
        let read = |error: std::io::Error| format!("reading the job: {error}");
        let spool = Folder::at(&self.spool).map_err(read)?;
        let folder = spool
            .below(&Path::new(JOBS).join(&name))
            .map_err(|error| format!("the job's folder is not a folder of its own: {error}"))?;
        let project = folder
            .folder(OsStr::new(PROJECT))
            .map_err(|error| format!("the job's project is not a folder of its own: {error}"))?;
        let cache = self
            .page_cache(&spool, &project)
            .ok_or("the job's project has no page cache of its own to capture into")?;
        let to = self.staged(&name);
        let _ = std::fs::remove_dir_all(&to);
        let write = |error: std::io::Error| format!("staging the job: {error}");
        std::fs::create_dir_all(&to).map_err(write)?;
        let mut staged = Staged {
            name,
            cache,
            media: BTreeSet::new(),
        };
        let copied = self.copy(&project, &to.join(PROJECT), Path::new(""), &mut staged);
        let asked = serde_json::to_vec(ask)
            .map_err(std::io::Error::other)
            .and_then(|bytes| std::fs::write(to.join(ASK), bytes))
            .map_err(write);
        if let Err(why) = copied.and(asked) {
            self.unstage(&staged);
            return Err(why);
        }
        Ok(staged)
    }

    /// Copies what capture `index` of `staged` wrote into the project's page
    /// cache, once its container is gone: files and folders only, each
    /// placed whole and none through a link. A leftover partial is left out:
    /// a capture cut short. What is neither a file nor a folder is said and
    /// left out.
    pub fn publish(&self, staged: &Staged, index: usize) -> Result<(), String> {
        let out = self.out(staged, index);
        let kept = Folder::at(&out).and_then(|from| {
            let (user, project) = &staged.cache;
            let to = Folder::at(&self.spool)?
                .folder_made(OsStr::new(PAGES))?
                .folder_made(OsStr::new(user))?
                .folder_made(OsStr::new(project))?;
            let mut refused = Vec::new();
            merge(&from, &to, Path::new(""), &mut refused)?;
            for path in refused {
                tracing::warn!(
                    "capture {index} of {} left {}, which is not a file or a folder; not kept",
                    staged.name,
                    path.display()
                );
            }
            Ok(())
        });
        let _ = std::fs::remove_dir_all(&out);
        kept.map_err(|error| format!("the capture could not be kept: {error}"))
    }

    /// Removes `staged` and what its captures left.
    pub fn unstage(&self, staged: &Staged) {
        let _ = std::fs::remove_dir_all(self.staged(&staged.name));
        let prefix = format!("{}-", staged.name);
        for entry in std::fs::read_dir(self.staging.join(OUT))
            .into_iter()
            .flatten()
        {
            let Ok(entry) = entry else { continue };
            if entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.strip_prefix(&prefix).is_some_and(is_number))
            {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }

    /// Where `name` is staged.
    pub(super) fn staged(&self, name: &str) -> PathBuf {
        self.staging.join(STAGED_JOBS).join(name)
    }

    /// Where capture `index` of `staged` writes.
    pub(super) fn out(&self, staged: &Staged, index: usize) -> PathBuf {
        self.staging
            .join(OUT)
            .join(format!("{}-{index}", staged.name))
    }

    /// Copies the folder `from` (at `at` in the project) to `to`: folders and
    /// files as they are, a link only when it is one of the owner's media,
    /// and the project's own `cache/` as an empty folder, where the capture's
    /// is mounted.
    fn copy(&self, from: &Folder, to: &Path, at: &Path, staged: &mut Staged) -> Result<(), String> {
        let write = |error: std::io::Error| format!("staging the job: {error}");
        std::fs::create_dir(to).map_err(write)?;
        let read = |error: std::io::Error| format!("reading the job: {error}");
        for (name, kind) in from.entries().map_err(read)? {
            let (path, target) = (at.join(&name), to.join(&name));
            if path == Path::new(CACHE_DIR) {
                std::fs::create_dir(&target).map_err(write)?;
                continue;
            }
            match kind {
                Kind::Folder => {
                    let below = from.folder(&name).map_err(read)?;
                    self.copy(&below, &target, &path, staged)?;
                }
                Kind::File => {
                    let mut file = from.file(&name).map_err(read)?;
                    std::fs::File::create_new(&target)
                        .and_then(|mut copy| std::io::copy(&mut file, &mut copy))
                        .map_err(write)?;
                }
                Kind::Link => {
                    let file = from
                        .link(&name)
                        .ok()
                        .filter(|link| self.is_media(link, &staged.cache.0))
                        .ok_or_else(|| {
                            format!(
                                "the project links {} to something that is not its owner's file",
                                path.display()
                            )
                        })?;
                    std::os::unix::fs::symlink(&file, &target).map_err(write)?;
                    staged.media.insert(file);
                }
                Kind::Other => {
                    return Err(format!(
                        "the project holds {}, which is not a file, a folder or a link",
                        path.display()
                    ));
                }
            }
        }
        Ok(())
    }

    /// Whether `link`, a link's target, is a file of `owner`'s:
    /// `<library>/users/<owner>/…`, written plainly, reached without a link
    /// on the way, and a file.
    fn is_media(&self, link: &Path, owner: &str) -> bool {
        let theirs = library_of(&self.library, owner);
        let Some(inside) = is_plain(link)
            .then(|| link.strip_prefix(&theirs).ok())
            .flatten()
        else {
            return false;
        };
        let (Some(parent), Some(name)) = (inside.parent(), inside.file_name()) else {
            return false;
        };
        // From the library the operator named: the owner's own folder is no
        // more to be followed than any other on the way.
        let parent = Path::new("users").join(owner).join(parent);
        Folder::at(&self.library)
            .and_then(|folder| folder.below(&parent))
            .and_then(|folder| folder.file(name))
            .is_ok()
    }

    /// Whose page cache the project open at `project` captures into, if its
    /// `cache/` is a link to one: exactly `<spool>/pages/<user>/<project>`, a
    /// folder, reached without a link on the way.
    fn page_cache(&self, spool: &Folder, project: &Folder) -> Option<(String, String)> {
        let target = project.link(OsStr::new(CACHE_DIR)).ok()?;
        let parts = cache_parts(&self.spool, &target)?;
        spool
            .below(&Path::new(PAGES).join(&parts.0).join(&parts.1))
            .ok()?;
        Some(parts)
    }
}

/// Copies every file and folder under `from` (at `at`) into `to`, each file
/// placed whole; a partial left out, a file already there as it is left as it
/// is, and what is neither added to `refused`. Each folder's records go last:
/// a record is what makes the frames beside it usable
/// (`scorsese_render::page`), so it never lands beside the older frames it
/// would vouch for.
fn merge(from: &Folder, to: &Folder, at: &Path, refused: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries = from.entries()?;
    entries.sort_by_key(|(name, _)| is_record(name));
    for (name, kind) in entries {
        match kind {
            Kind::Folder => {
                let below = from.folder(&name)?;
                merge(&below, &to.folder_made(&name)?, &at.join(&name), refused)?;
            }
            Kind::File if is_partial(&name) => {}
            Kind::File => {
                let mut file = from.file(&name)?;
                if !to.holds_same(&name, &mut file)? {
                    to.place(&name, &mut file)?;
                }
            }
            Kind::Link | Kind::Other => refused.push(at.join(&name)),
        }
    }
    Ok(())
}

/// A capture's record (`capture.json`, `part-<a>-<b>.json`).
fn is_record(name: &OsStr) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|extension| extension == "json")
}

/// A capture's frames written beside their place and never moved there:
/// `<stem>.<pid>.<n>.partial.mkv` (`scorsese_render::page`), a capture
/// stopped at its deadline.
fn is_partial(name: &OsStr) -> bool {
    name.to_str()
        .is_some_and(|name| name.ends_with(".partial.mkv"))
}

#[cfg(test)]
mod tests;
