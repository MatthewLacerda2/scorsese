//! Page captures on the web app (#778): a page drawn by a browser in a
//! container of its own, offline, and handed back as frames in `cache/`.
//!
//! On the web a page is **somebody else's code on the maintainer's machine**,
//! and the machine serves other people. So the browser never runs in the
//! server's container, which holds the database's password and reaches the
//! internet. Each capture runs in a container of its own (#852): no network at
//! all, no capability, a read-only root, Chromium's own sandbox on
//! ([`scorsese_render::page::Chrome::sandboxed`]) with the one seccomp rule
//! #773 found it needs, #773's caps on CPU, memory and processes — and
//! **nothing mounted but its own job**: the project, read-only, the media
//! files that project links to, read-only, and its page cache.
//!
//! ## How the server asks: a spool, and a launcher holding the socket
//!
//! Starting a container takes the Docker socket, which is root on the host.
//! The server is the one container the internet talks to, so it never holds
//! it. A small sidecar does, `capture-launcher` (`deploy/compose.yaml`): it
//! has no network, no database and no secret, and the only container it can
//! start is the one [`launch`] writes out, flag for flag. Between the two is a
//! **folder both mount**:
//!
//! - `jobs/<job>/project.scor/`: the project, laid out by the render job
//!   exactly as for a render ([`crate::projects::media::materialise`]), kept
//!   pages included. Its `cache/` is a link to the project's page cache, so a
//!   capture outlives the render that asked for it.
//! - `jobs/<job>/ask.json`: what to capture ([`Ask`]), written last and moved
//!   into place, so a job the launcher can see is a whole one.
//! - `jobs/<job>/answer.json`: what happened to each capture ([`Answer`]),
//!   written by the launcher the same way.
//! - `pages/<user>/<project>/`: each project's page cache, the `cache/` its
//!   captures land in. Per user and per project, so a user's captured frames
//!   are never another's.
//! - `worker.alive`: when the launcher last looked, so a server whose launcher
//!   is not running says so rather than waiting forever.
//!
//! The launcher takes one capture at a time (#773: one wants ~3.6 cores), and
//! that is the captures' per-kind limit: a render whose project has pages
//! waits for it in its own slot, since it cannot be drawn without them.
//!
//! **Two walls, not one.** Inside its container a page's renderer can open no
//! file at all (Chromium's sandbox), and the capture's origin serves it only
//! files under its project root, or under the owner's library where the
//! project's media links lead (`--follow`, #857; `scorsese_render::page`). A
//! page that got past
//! both would still find nothing of anybody else's: no other project, no other
//! user's library, no other job. #778 ran one long-lived worker instead, which
//! saw every user's library and the whole spool; #852 is why it does not.
//!
//! **The deadline is kept from outside the browser**: each capture is killed,
//! container and all, at 60 s plus a second a frame ([`deadline`], #773). The
//! capture's own patience stops a page that never answers; this stops one that
//! answers slowly forever.
//!
//! Nothing here edits anything: the job lays the project out and renders it,
//! and each capture calls [`scorsese_render::page::capture`] on what it is asked.

pub mod dispatch;
pub mod launch;
pub mod one;
pub mod worker;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use scorsese_core::Fps;
use scorsese_render::Resolution;
use scorsese_render::page::{Request, Span};
use serde::{Deserialize, Serialize};

use crate::db::UserId;

/// The folder the spool lives in, under `SCORSESE_CACHE`.
pub const SPOOL_DIR: &str = "captures";

/// The capture's own deadline: [`BASE`] plus [`PER_FRAME`] for each frame.
pub const BASE: Duration = Duration::from_secs(60);

/// What each frame adds to [`BASE`]: ~4× the capped cost #773 measured.
pub const PER_FRAME: Duration = Duration::from_secs(1);

/// How long [`deadline`] gives a capture of `frames` frames.
pub fn deadline(frames: u64) -> Duration {
    BASE + PER_FRAME * u32::try_from(frames).unwrap_or(u32::MAX)
}

/// The spool both containers mount, at the same path in each.
#[derive(Debug, Clone)]
pub struct Spool {
    root: PathBuf,
}

const JOBS: &str = "jobs";
const PAGES: &str = "pages";
const ASK: &str = "ask.json";
const ANSWER: &str = "answer.json";
const ALIVE: &str = "worker.alive";
/// The project's folder inside a job's.
pub const PROJECT: &str = "project.scor";

impl Spool {
    /// A spool at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Its folder.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Job `job`'s folder.
    pub fn job(&self, job: i64) -> PathBuf {
        self.root.join(JOBS).join(format!("job-{job}"))
    }

    /// Where `user`'s project `project` keeps its captures.
    pub fn pages(&self, user: UserId, project: i64) -> PathBuf {
        self.root
            .join(PAGES)
            .join(user.get().to_string())
            .join(project.to_string())
    }

    /// Where the jobs' folders are.
    pub fn jobs(&self) -> PathBuf {
        self.root.join(JOBS)
    }

    fn alive(&self) -> PathBuf {
        self.root.join(ALIVE)
    }
}

/// One capture asked for: a [`Request`] as it travels between containers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Asked {
    /// The page's path from the project root.
    pub page: String,
    /// The raster's width.
    pub width: u32,
    /// The raster's height.
    pub height: u32,
    /// The rate.
    pub fps: Fps,
    /// How far the page's clock runs, in seconds.
    pub duration: f64,
    /// Where the clips beside the page sit, in its seconds. Absent from a job
    /// asked before pages were told (#810), which told them none.
    #[serde(default)]
    pub clips: BTreeMap<String, Span>,
}

impl From<&Request> for Asked {
    fn from(request: &Request) -> Self {
        Self {
            page: request.page.clone(),
            width: request.resolution.width(),
            height: request.resolution.height(),
            fps: request.fps,
            duration: request.duration,
            clips: request.clips.clone(),
        }
    }
}

impl Asked {
    /// The request again, or why it is not one.
    pub fn request(&self) -> Result<Request, String> {
        Ok(Request {
            page: self.page.clone(),
            resolution: Resolution::new(self.width, self.height)
                .map_err(|error| error.to_string())?,
            fps: self.fps,
            duration: self.duration,
            clips: self.clips.clone(),
        })
    }
}

/// What a job asks the worker to capture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ask {
    /// Each capture, in order.
    pub requests: Vec<Asked>,
}

/// What became of each capture, in the order asked.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Answer {
    /// `None` for a capture now in the project's cache, or why it is not.
    pub failed: Vec<Option<String>>,
}

/// Writes `value` to `path` whole: beside it first, then moved into place.
fn publish(path: &Path, value: &impl Serialize) -> std::io::Result<()> {
    let partial = path.with_extension("partial");
    let bytes = serde_json::to_vec_pretty(value).map_err(std::io::Error::other)?;
    std::fs::write(&partial, bytes)?;
    std::fs::rename(&partial, path)
}

/// Reads the JSON at `path`, if it is there and whole.
fn read<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Seconds since the epoch, as both containers' shared clock tells it.
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_survives_the_trip_between_containers() {
        let request = Request {
            page: "pages/title.html".into(),
            resolution: Resolution::new(1920, 1080).unwrap(),
            fps: Fps::PAL,
            duration: 2.5,
            clips: BTreeMap::from([(
                "vo".to_owned(),
                Span {
                    start: -0.5,
                    end: 1.25,
                },
            )]),
        };
        let asked = Asked::from(&request);
        let back: Asked = serde_json::from_str(&serde_json::to_string(&asked).unwrap()).unwrap();
        assert_eq!(back.request().unwrap(), request);
    }

    #[test]
    fn the_deadline_is_a_minute_and_a_second_a_frame() {
        assert_eq!(deadline(0), Duration::from_secs(60));
        assert_eq!(deadline(1800), Duration::from_secs(1860));
    }

    #[test]
    fn a_users_captures_are_kept_apart_by_user_and_project() {
        let spool = Spool::new("/c");
        let one = UserId::from_row(1);
        assert_ne!(spool.pages(one, 7), spool.pages(UserId::from_row(2), 7));
        assert_ne!(spool.pages(one, 7), spool.pages(one, 8));
    }
}
