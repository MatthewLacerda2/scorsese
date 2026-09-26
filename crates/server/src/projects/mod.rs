//! A user's projects, stored in Postgres and edited by `scorsese-core` (#534).
//!
//! ## The edit is a document
//!
//! A project row holds the **whole `project.json`** in a `JSONB` column. It is
//! read into [`scorsese_core::Project`], changed by the same functions the CLI
//! and MCP call, and written back whole. Nothing here knows what a clip is:
//! the timeline is not normalised into tables, because the format is
//! versioned and changes often, and normalising it would be a second
//! implementation of every edit, in SQL, drifting from the first
//! (`docs/web.md`, *The edit is a document*).
//!
//! `JSONB` rather than `JSON` because nothing here depends on the bytes: key
//! order and whitespace are not meaning, and every write re-serialises the
//! document anyway. What `JSONB` buys is a document Postgres can look inside —
//! the generated `name` column, and the startup query below that finds the
//! documents written by an older build.
//!
//! ## Two writers never silently overwrite each other
//!
//! The browser, the built-in assistant and a user's own MCP client can all
//! hold the same project at once. **Every row carries a `revision`**, a number
//! that goes up by one on each write; [`save`] names the revision the document
//! was read at, and is refused with [`ProjectError::Conflict`] when it is no
//! longer the current one. The check and the write are one `UPDATE … WHERE
//! revision = $n`, so unlike the file-based guard a `.scor` folder has
//! (`scorsese_core::Baseline`) there is no gap between them: Postgres makes
//! the second of two racing writers re-read the row, find the revision moved,
//! and update nothing.
//!
//! A number rather than a content fingerprint: it is what a client already
//! has to carry back, it cannot collide, and it does not depend on the stored
//! bytes — which `JSONB` does not keep anyway. And a check rather than a lock:
//! a lock held across *read → think → write* is an assistant holding a mutex
//! for a minute, and a project wedged when one dies.
//!
//! [`edit`] is the other way in, for a change that takes no time — a rename:
//! read, change and write inside one transaction with the row locked, so there
//! is nothing to conflict with.
//!
//! ## Which files a project uses
//!
//! `project_assets` holds one row per distinct `sha256` in the document's
//! assets table, **rewritten from the document on every write** and never any
//! other way, so it answers "which projects use this file?" — the question the
//! library (#535) asks before letting a file be deleted — without anything
//! having to remember to keep it current. See [`media`] for the path a
//! library file takes inside a stored document, and for rendering one.
//!
//! ## A schema bump migrates every stored document
//!
//! [`migrate_stored`] runs when the server starts, before it serves: every
//! document written by an older build is carried forward by
//! `scorsese_core::migrate` — the same steps `scorsese migrate` runs over a
//! local folder — in one transaction, so either every project is readable by
//! this build or the server does not start.

pub mod media;
mod startup;
mod store;

pub use startup::migrate_stored;
pub use store::{create, delete, edit, list, open, save};

use scorsese_core::migrate::MigrateError;
use scorsese_core::{LoadError, Project};
use serde::Serialize;

/// A project as a list shows it: everything but the document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Summary {
    /// What to open it by.
    pub id: i64,
    /// The document's own `name`.
    pub name: String,
    /// What a save of a document read now has to name.
    pub revision: i64,
    /// When it was created, in seconds since the Unix epoch.
    pub created_at: i64,
    /// When it was last written.
    pub updated_at: i64,
}

/// A project opened for editing: the row, and its document.
#[derive(Debug, Clone, Serialize)]
pub struct Stored {
    /// Everything but the document.
    #[serde(flatten)]
    pub summary: Summary,
    /// The edit itself.
    pub document: Project,
}

/// Why a project operation did not happen.
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    /// The user has no project by that id — including when somebody else
    /// does, which is indistinguishable on purpose.
    #[error("there is no such project")]
    NotFound,

    /// The document was based on a revision that is no longer current.
    #[error(
        "the project changed since it was read (it is at revision {current}); \
         read it again and redo the edit on what is there now"
    )]
    Conflict {
        /// The revision the project is at now.
        current: i64,
    },

    /// A stored document does not read as a project — which the startup
    /// migration exists to make impossible.
    #[error("stored project {id} does not load: {source}")]
    Unreadable {
        /// Which project.
        id: i64,
        /// Why.
        #[source]
        source: LoadError,
    },

    /// A stored document could not be carried forward to this build.
    #[error("stored project {id} could not be migrated: {source}")]
    Migrate {
        /// Which project.
        id: i64,
        /// Why.
        #[source]
        source: MigrateError,
    },

    /// The document names files its owner's library does not hold.
    #[error(
        "{} {} a file that is not in your library; add it to the library first, \
         or take {} out of the project",
        .assets.join(", "),
        if .assets.len() == 1 { "names" } else { "name" },
        if .assets.len() == 1 { "that asset" } else { "those assets" }
    )]
    UnknownFiles {
        /// The assets, by id.
        assets: Vec<String>,
    },

    /// The document could not be serialised — a non-finite number in it.
    #[error("serialising the project: {0}")]
    Serialize(#[from] serde_json::Error),

    /// The database refused or could not be reached.
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
}
