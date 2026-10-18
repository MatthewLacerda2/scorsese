//! A still's pages, captured by the launcher before the still is drawn (#1434).
//!
//! `still` on the web is the registry's own tool, run on the project laid out
//! in a scratch folder (`crate::tools::stored`). Left to itself it would start
//! a browser in this container, which is the one thing [`super`] says never
//! happens here — and which failed anyway, for want of a sandbox, so every
//! page on the web was looked at as its slug card. So the tool is handed
//! [`StillPages`] ([`scorsese_mcp::PagesAhead`]) and draws without capturing:
//!
//! - The tool folder's `cache/pages` is a link to the project's page cache, so
//!   a page a render or an earlier still captured is read from there, and a
//!   still whose pages are all there asks for nothing.
//! - Otherwise the folder is copied into the spool as a job of its own,
//!   `still-<call>` ([`Spool::still`]), and asked exactly as a render job asks
//!   ([`dispatch`]): the same launcher, the same container, the same walls.
//! - It waits as long as the launcher would give the capture itself
//!   ([`super::deadline`]), but never past [`PATIENCE`]: the call is somebody
//!   waiting behind nginx, which gives up at 300 s (#1469). One capture runs
//!   at a time, so a still may wait behind a render's pages; past that
//!   patience it is drawn with the page as "being captured" (`underway`,
//!   #1202), and the job is left for the launcher to finish into the cache,
//!   where the next still finds it — for at most [`LATER`].
//! - A call stopped while it waits stops waiting (#1469), and its job is left
//!   to finish the same way: somebody who stopped a look usually looks again.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use scorsese_core::CACHE_DIR;
use scorsese_mcp::{Ahead, PagesAhead};
use scorsese_render::Cancel;
use scorsese_render::page::{self, Wanted};

use super::dispatch::{self, NOT_RUNNING, Waited};
use super::{ASK, Ask, Asked, PROJECT, Spool, deadline, publish};
use crate::db::UserId;
use crate::renders::{Pin, RenderCache};

/// The longest a still waits for its pages: well inside nginx's 300 s
/// `proxy_read_timeout`, with the drawing still to come after it, and a wait
/// nobody looking at a frame would sit through much longer anyway.
const PATIENCE: Duration = Duration::from_secs(90);

/// The longest a still's job is left to the launcher once nobody waits on it.
/// The launcher answers in the order asked, so a still may be queued behind a
/// render's whole capture — but a job never answered (a launcher wedged, a
/// folder gone) must not hold the page cache's pin forever.
const LATER: Duration = Duration::from_secs(60 * 60);

/// Why a page is a card while the launcher is still at it.
const UNDERWAY: &str = "it is being captured now, behind the other captures this server is \
                        making; look again in a minute and it is there";

/// The page captures one tool call's still may ask for.
pub struct StillPages {
    /// The spool.
    pub spool: Spool,
    /// The project's page cache ([`Spool::pages`]).
    pub cache: PathBuf,
    /// The tool call asking, which names its job ([`Spool::still`]).
    pub call: i64,
    /// Keeps the page cache from eviction while it is read or captured into.
    pub pin: Arc<Pin>,
}

impl PagesAhead for StillPages {
    fn capture(&self, project_root: &Path, wanted: &[Wanted], cancel: &Cancel) -> Ahead {
        let chrome = match dispatch::browser() {
            Ok(chrome) => chrome,
            Err(why) => return failed(None, wanted, &why),
        };
        if let Err(error) = read_cache(project_root, &self.cache) {
            let why = format!("the page cache could not be prepared: {error}");
            return failed(Some(chrome), wanted, &why);
        }
        let missing: Vec<&Wanted> = wanted
            .iter()
            .filter(|one| {
                let frames = one.frames.clone();
                page::cached_frames(project_root, &one.request, chrome.version(), frames).is_none()
            })
            .collect();
        if missing.is_empty() {
            return Ahead {
                chrome: Some(chrome),
                ..Ahead::default()
            };
        }
        // Not asked at all: a still is somebody looking, and thirty seconds
        // of waiting to learn the launcher is down is thirty seconds wasted.
        if !dispatch::worker_alive(&self.spool) {
            return failed(Some(chrome), wanted, NOT_RUNNING);
        }
        let job = self.spool.still(self.call);
        if let Err(why) = self.ask(project_root, &job, &missing) {
            let _ = std::fs::remove_dir_all(&job);
            return failed(Some(chrome), wanted, &why);
        }
        let frames: u64 = missing.iter().map(|one| one.frames.end).sum();
        let patience = deadline(frames).min(PATIENCE);
        let waited = dispatch::wait(&self.spool, &job, cancel, Some(patience));
        match waited {
            Ok(Waited::Answered(answer)) => {
                let _ = std::fs::remove_dir_all(&job);
                let failed = missing
                    .iter()
                    .zip(answer.failed)
                    .filter_map(|(one, why)| Some((one.request.page.clone(), why?)))
                    .collect();
                Ahead {
                    chrome: Some(chrome),
                    failed,
                    underway: false,
                }
            }
            Ok(Waited::Late) => {
                self.finish_later(job);
                let mut ahead = failed(Some(chrome), missing.iter().copied(), UNDERWAY);
                ahead.underway = true;
                ahead
            }
            Err(why) => {
                self.finish_later(job);
                failed(Some(chrome), missing.iter().copied(), &why)
            }
            Ok(Waited::Quiet | Waited::Gone) => {
                let _ = std::fs::remove_dir_all(&job);
                failed(Some(chrome), wanted, NOT_RUNNING)
            }
        }
    }
}

impl StillPages {
    /// The captures tool call `call` of `user`'s may ask for, of `project`'s
    /// pages, the page cache held from now until they are drawn.
    pub async fn of(renders: &RenderCache, user: UserId, project: i64, call: i64) -> Arc<Self> {
        let spool = renders.captures();
        Arc::new(Self {
            cache: spool.pages(user, project),
            spool,
            call,
            pin: Arc::new(crate::renders::evict::hold(renders, user, project).await),
        })
    }

    /// Lays the project at `project_root` out as the job at `job`, its cache
    /// the project's, and asks for `missing`.
    fn ask(&self, project_root: &Path, job: &Path, missing: &[&Wanted]) -> Result<(), String> {
        let _ = std::fs::remove_dir_all(job);
        let folder = job.join(PROJECT);
        copy_project(project_root, &folder)
            .and_then(|()| dispatch::share_cache(&folder, &self.cache))
            .map_err(|error| format!("the page cache could not be prepared: {error}"))?;
        let ask = Ask {
            requests: missing.iter().map(|one| Asked::from(*one)).collect(),
        };
        publish(&job.join(ASK), &ask)
            .map_err(|error| format!("the page renderer could not be asked: {error}"))
    }

    /// Leaves the job at `job` to the launcher, and removes it once answered,
    /// or once it has waited [`LATER`]: withdrawn, the launcher skips it.
    fn finish_later(&self, job: PathBuf) {
        let (spool, pin) = (self.spool.clone(), Arc::clone(&self.pin));
        std::thread::spawn(move || {
            let _ = dispatch::wait(&spool, &job, &Cancel::new(), Some(LATER));
            let _ = std::fs::remove_dir_all(&job);
            drop(pin);
        });
    }
}

/// Every page in `wanted` failed, for `why`.
fn failed<'a>(
    chrome: Option<page::Chrome>,
    wanted: impl IntoIterator<Item = &'a Wanted>,
    why: &str,
) -> Ahead {
    Ahead {
        chrome,
        failed: wanted
            .into_iter()
            .map(|one| (one.request.page.clone(), why.to_owned()))
            .collect::<HashMap<_, _>>(),
        underway: false,
    }
}

/// Points the page captures of the project laid out at `root` at the
/// project's page cache `kept`, leaving the rest of its `cache/` its own.
fn read_cache(root: &Path, kept: &Path) -> std::io::Result<()> {
    // The page cache is what a job's `cache/` links to, so its captures are
    // in the same folder under it that they are under `cache/`.
    let here = root.join(CACHE_DIR).join(page::CACHE_PAGES);
    let there = kept.join(page::CACHE_PAGES);
    std::fs::create_dir_all(&there)?;
    if here.symlink_metadata().is_ok() {
        std::fs::remove_dir_all(&here)?;
    }
    if let Some(parent) = here.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::os::unix::fs::symlink(std::path::absolute(&there)?, &here)
}

/// Copies the project laid out at `from` to `to`, its links as links and
/// without its `cache/`, which the job gets as the project's own.
fn copy_project(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let (source, target) = (entry.path(), to.join(entry.file_name()));
        if from.join(CACHE_DIR) == source {
            continue;
        }
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            std::os::unix::fs::symlink(std::fs::read_link(&source)?, &target)?;
        } else if kind.is_dir() {
            copy_project(&source, &target)?;
        } else {
            std::fs::copy(&source, &target)?;
        }
    }
    Ok(())
}
