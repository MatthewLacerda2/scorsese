//! A user's library: every file they uploaded or generated, reusable in any of
//! their projects (#535).
//!
//! ## Stored once per (user, hash)
//!
//! A file is its SHA-256. It sits on disk at
//! `users/<id>/library/<sha256>.<extension>` under the storage root
//! ([`Storage::library_file`](crate::storage::Storage::library_file)), and in a
//! stored project at `assets/<sha256>.<extension>`
//! ([`library_path`](crate::projects::media::library_path)) — so the same file
//! is the same bytes in every project, and there is no path column that could
//! disagree with the hash. A byte-identical second upload is **refused**,
//! whatever its name, with the name it is already there under
//! ([`LibraryError::Duplicate`]): the browser hashes a file before sending it
//! and asks (`GET /api/library?sha256=`), so a duplicate normally never crosses
//! the network; the refusal at upload is the backstop.
//!
//! **The server never trusts a client's hash.** Whatever arrives is hashed
//! again ([`Library::admit`]) and must match what was announced; a file is also probed
//! and checked against its kind exactly as `scorsese import` checks one, by the
//! same `scorsese_core::pool` functions, so the web app accepts exactly the
//! files the CLI does — no more, no fewer.
//!
//! ## Every file a project names is a library item
//!
//! `project_assets` has a foreign key to this table (migration `0005`), so a
//! stored document cannot name a file its owner does not have, and an item a
//! project uses cannot be deleted. [`delete`] refuses first, naming the
//! projects, and the key is what holds if two requests race. Generated media is
//! included: a Veo shot, a spoken line and a synthesis bake are items too.
//!
//! ## Generated output is found by its brief
//!
//! A generated item carries the hash of the brief it was made from
//! ([`Library::find_generated`], [`Library::keep_generated`]). Asking for the same brief again in
//! another of the **same user's** projects finds it and costs nothing; the
//! lookup runs scoped, so it cannot find another user's. Two different briefs
//! that happen to produce the same bytes are one item under the first brief.
//!
//! ## Uploads, thumbnails
//!
//! [`upload`] is the tus protocol's storage half: chunked and resumable,
//! because the Cloudflare tunnel refuses a request body over 100 MB.
//! [`thumbnail`] is the job that draws each item's picture — the first handler
//! the job queue runs — into the cache, since a thumbnail can always be drawn
//! again.

mod admit;
mod generated;
mod kind;
mod store;
pub mod thumbnail;
pub mod upload;

pub use admit::Arrival;
pub use kind::Kind;
pub use store::{Change, Filter};
pub use upload::{Announced, Appended, MAX_UPLOAD_BYTES, Progress};

use scorsese_core::MediaMetadata;
use scorsese_render::Tools;
use serde::Serialize;
use sqlx::postgres::PgPool;

use crate::jobs::Queue;
use crate::storage::Storage;

/// A handle on every user's library: the database, where the files are, the
/// tools that read them and the queue their thumbnails wait in. Every method
/// acts for one user and runs scoped as them. Cheap to clone.
#[derive(Clone)]
pub struct Library {
    pool: PgPool,
    storage: Storage,
    tools: Tools,
    queue: Queue,
    uploading: upload::InFlight,
}

impl Library {
    /// The library kept in `storage`, read with `tools`, recorded in `pool`
    /// (a [`member_pool`](crate::db::member_pool)) and announcing its
    /// thumbnail jobs on `queue`.
    pub fn new(pool: PgPool, storage: Storage, tools: Tools, queue: Queue) -> Self {
        Self {
            pool,
            storage,
            tools,
            queue,
            uploading: upload::InFlight::default(),
        }
    }

    /// Where the files are.
    pub fn storage(&self) -> &Storage {
        &self.storage
    }
}

/// One file in a user's library, with everything known about it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Item {
    /// What to open it by.
    pub id: i64,
    /// What the user calls it.
    pub name: String,
    /// What it is.
    pub kind: Kind,
    /// Its content hash: what identifies it.
    pub sha256: String,
    /// Its file extension, lower case, without the dot.
    pub extension: String,
    /// Its size.
    pub size_bytes: i64,
    /// What probing it found, as a project's assets table records it.
    pub media: MediaMetadata,
    /// Words for the assistant to read when choosing a file.
    pub description: Option<String>,
    /// The hash of the brief it was generated from; `None` for an upload.
    pub brief_hash: Option<String>,
    /// When it arrived, in seconds since the Unix epoch.
    pub created_at: i64,
    /// When a project last saved with it in.
    pub last_used_at: Option<i64>,
}

/// An item as a list shows it: enough to draw a tile, and no more.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct Summary {
    /// What to open it by.
    pub id: i64,
    /// What the user calls it.
    pub name: String,
    /// What it is.
    #[sqlx(try_from = "String")]
    pub kind: Kind,
    /// Its size.
    pub size_bytes: i64,
}

/// A project, as the reason an item cannot be deleted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct UsedBy {
    /// The project's id.
    pub id: i64,
    /// Its name.
    pub name: String,
}

/// Why a library operation did not happen.
#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    /// The user has no item by that id — including when somebody else does.
    #[error("there is no such file in your library")]
    NotFound,

    /// The same bytes are already in the library.
    #[error("you already have this as \u{201c}{name}\u{201d}")]
    Duplicate {
        /// The item that has them.
        id: i64,
        /// What it is called.
        name: String,
    },

    /// Projects use the item, so it stays.
    #[error("{} uses this file; take it out of {} first", names(.projects), if .projects.len() == 1 { "that project" } else { "those projects" })]
    InUse {
        /// Which.
        projects: Vec<UsedBy>,
    },

    /// Not a kind of file scorsese edits, by its name.
    #[error("scorsese cannot use {0:?}: it takes video, pictures and sound")]
    Unsupported(String),

    /// The file is not what it said it was — a hash that differs from the one
    /// announced, a `.mp4` with no picture, a file no prober reads.
    #[error("{0}")]
    Rejected(String),

    /// A name, a size or a hash that is not one.
    #[error("{0}")]
    Invalid(String),

    /// Another request is writing this upload right now. Try again shortly.
    #[error("this upload is being written by another request; try again shortly")]
    Busy,

    /// A chunk that does not start where the upload got to.
    #[error("the upload has {expected} bytes so far, and this chunk does not start there")]
    Offset {
        /// Where the next chunk has to start.
        expected: i64,
    },

    /// The disk refused. The detail is for the log.
    #[error("storage: {0}")]
    Io(#[from] std::io::Error),

    /// ffmpeg or ffprobe could not be run at all.
    #[error("media tools: {0}")]
    Tools(String),

    /// The database refused or could not be reached.
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
}

/// `“a”, “b” and “c”`.
fn names(projects: &[UsedBy]) -> String {
    let quoted: Vec<String> = projects
        .iter()
        .map(|project| format!("\u{201c}{}\u{201d}", project.name))
        .collect();
    match quoted.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        _ => quoted.concat(),
    }
}
