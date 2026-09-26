//! Finished renders: made by the job queue, kept for download, and deleted
//! again by one deliberately simple rule (#541).
//!
//! ## One render per (document, settings)
//!
//! A render is keyed by a hash of **the project's document and the render
//! settings** ([`settings::key`]), so asking again for an unchanged project in
//! the same shape answers with the file already made, instantly. The document
//! names every file it uses by `sha256`, so a changed file is a changed
//! document and a new key; nothing else goes into a render. Rows are unique
//! per (project, key): two projects that happen to hold the same document
//! render once each, which is rare and costs only disk.
//!
//! The settings are the ones `docs/output-formats.md` allows and nothing more
//! — a container, its two codecs and, for a picture, a resolution — built by
//! the same [`OutputFormat::new`](scorsese_render::OutputFormat::new) the CLI
//! and MCP use, so every refusal reads the same from the web. The frame rate
//! is the project's own, as `scorsese render` defaults to, and the whole
//! timeline is rendered.
//!
//! ## Where they live
//!
//! **`<SCORSESE_CACHE>/users/<user>/renders/<project>/<key>.<ext>`**: under the
//! cache root, never under the library's, because a render can always be made
//! again from its stored project and is not worth a place in the nightly
//! backup (`docs/web.md`, *Running the service*). The row keeps that path
//! relative to the root. A render is produced in a scratch folder
//! ([`RenderCache::work`]) and moved into place only when it is complete, so a
//! half-written file is never downloadable, and that folder is emptied when
//! the server starts, since no job is running then.
//!
//! ## Eviction: the maintainer's rule
//!
//! The operator sets a quota (`SCORSESE_RENDER_QUOTA`). When a new render
//! would take the cache past it, **every render not used in the last
//! [`IDLE`] is deleted** — every user's, which makes it one of the few
//! privileged steps (`db::scope`). A download counts as use, and so does
//! asking for a render that is already there. If nothing is that old the new
//! render is kept anyway and a warning is logged: **the quota is a target,
//! not a wall.** A weekly sweep ([`evict::run`]) applies the same rule
//! whatever the pressure, and removes files no row names any more — a deleted
//! project's or a deleted account's. Deleting a render is always safe,
//! because it can be rebuilt from the stored project; that is why the rule can
//! be this simple.
//!
//! ## Never deleted mid-download
//!
//! Two things, because there is one server process and so one place every
//! download and every deletion passes through:
//!
//! - **A download stamps the row as used before it opens the file**, in its
//!   own committed transaction. Eviction deletes with `last_used_at` older
//!   than [`IDLE`] in the `DELETE` itself, and Postgres re-checks that against
//!   the committed stamp, so a file somebody has started downloading is not a
//!   candidate at all.
//! - **Every open file is pinned** ([`RenderCache::pin`]): an in-process count
//!   per path, held until the response body is dropped. Eviction and the sweep
//!   skip anything pinned. That covers what the stamp cannot — a download
//!   still running [`IDLE`] after it started, or a project deleted while its
//!   render streams.
//!
//! A count in memory rather than a column: it describes this process's open
//! files, which a crash closes, and a column would need clearing after one.

pub mod evict;
pub mod job;
pub mod settings;
pub mod store;

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use serde::Serialize;

pub use settings::{Ask, Settings, key};

use crate::db::UserId;

/// How long a render may go unused before eviction may take it: 48 hours.
pub const IDLE: Duration = Duration::from_secs(48 * 60 * 60);

/// A size in bytes, as the operator writes it: `20GB`, `500MB`, `1TB`, or a
/// bare number of bytes. Decimal units, as disks are sold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quota(u64);

impl Quota {
    /// A quota of exactly `bytes`.
    pub const fn bytes(bytes: u64) -> Self {
        Self(bytes)
    }

    /// How many bytes.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl FromStr for Quota {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, ()> {
        let text = text.trim();
        let digits = text
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(text.len());
        let (number, unit) = text.split_at(digits);
        let scale: u64 = match unit.trim().to_ascii_uppercase().as_str() {
            "" | "B" => 1,
            "KB" => 1_000,
            "MB" => 1_000_000,
            "GB" => 1_000_000_000,
            "TB" => 1_000_000_000_000,
            _ => return Err(()),
        };
        let number: u64 = number.parse().map_err(|_| ())?;
        number.checked_mul(scale).map(Self).ok_or(())
    }
}

impl fmt::Display for Quota {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} bytes", self.0)
    }
}

/// The render cache: where finished renders are, the quota they aim for, and
/// which of them are open right now. Cheap to clone; clones share the pins.
#[derive(Debug, Clone)]
pub struct RenderCache {
    root: PathBuf,
    quota: Quota,
    pins: Arc<Mutex<HashMap<PathBuf, usize>>>,
    /// Held while a render is admitted or the cache swept, so two finishing
    /// renders do not both count the cache before either is in it, and the
    /// sweep never takes a file moved into place a moment before its row.
    admitting: Arc<tokio::sync::Mutex<()>>,
}

impl RenderCache {
    /// A cache under `root` (`SCORSESE_CACHE`) aiming to stay under `quota`.
    pub fn new(root: impl Into<PathBuf>, quota: Quota) -> Self {
        Self {
            root: root.into(),
            quota,
            pins: Arc::default(),
            admitting: Arc::default(),
        }
    }

    /// The quota.
    pub fn quota(&self) -> Quota {
        self.quota
    }

    /// Create the root, and empty the scratch folder a stopped server left.
    /// Only while no render runs — the server calls it before its worker.
    /// On failure, the directory with the error, so the caller can name it.
    pub fn prepare(&self) -> Result<(), (PathBuf, std::io::Error)> {
        std::fs::create_dir_all(&self.root).map_err(|error| (self.root.clone(), error))?;
        let work = self.root.join(WORK);
        match std::fs::remove_dir_all(&work) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err((work, error)),
            _ => Ok(()),
        }
    }

    /// Where `user`'s render of `project` with `key` is kept, relative to the
    /// root — what the row stores.
    pub fn relative(user: UserId, project: i64, key: &str, extension: &str) -> PathBuf {
        Path::new(USERS)
            .join(user.get().to_string())
            .join(RENDERS)
            .join(project.to_string())
            .join(format!("{key}.{extension}"))
    }

    /// A path the row stores, on disk.
    pub fn absolute(&self, relative: &Path) -> PathBuf {
        self.root.join(relative)
    }

    /// The scratch folder job `job` renders in.
    pub fn work(&self, job: i64) -> PathBuf {
        self.root.join(WORK).join(format!("job-{job}"))
    }

    /// Keep the file at `relative` from being deleted until the pin drops.
    pub fn pin(&self, relative: &Path) -> Pin {
        *self.pinned().entry(relative.to_path_buf()).or_default() += 1;
        Pin {
            pins: Arc::clone(&self.pins),
            path: relative.to_path_buf(),
        }
    }

    /// Every path pinned right now.
    fn pinned_paths(&self) -> Vec<PathBuf> {
        self.pinned().keys().cloned().collect()
    }

    fn pinned(&self) -> std::sync::MutexGuard<'_, HashMap<PathBuf, usize>> {
        // A count is still a count after a panic elsewhere held the lock.
        self.pins.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The folder under the root renders are produced in.
const WORK: &str = "renders-work";
/// Per-user directories under the root, as for the library.
const USERS: &str = "users";
/// Each user's renders, under their directory.
const RENDERS: &str = "renders";

/// A render file somebody has open. Dropping it lets eviction have the file.
#[derive(Debug)]
pub struct Pin {
    pins: Arc<Mutex<HashMap<PathBuf, usize>>>,
    path: PathBuf,
}

impl Drop for Pin {
    fn drop(&mut self) {
        let mut pins = self.pins.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(count) = pins.get_mut(&self.path) {
            *count -= 1;
            if *count == 0 {
                pins.remove(&self.path);
            }
        }
    }
}

/// A finished render, as its owner sees it.
#[derive(Debug, Clone, PartialEq, Serialize, sqlx::FromRow)]
pub struct RenderView {
    /// Its id.
    pub id: i64,
    /// The project it is a render of.
    pub project: i64,
    /// The hash of the document and settings it was made from.
    pub key: String,
    /// What shape of file it is.
    #[sqlx(json)]
    pub settings: Settings,
    /// Its size in bytes.
    pub size: i64,
    /// When it was made, in seconds since the Unix epoch.
    pub created_at: i64,
    /// When it was last downloaded or asked for.
    pub last_used_at: i64,
}

impl RenderView {
    /// Where to download it.
    pub fn file(&self) -> String {
        format!("/api/renders/{}/file", self.id)
    }
}
