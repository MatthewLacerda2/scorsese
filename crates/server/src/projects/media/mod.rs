//! The files a stored project uses: how its document names them, how the
//! database remembers which they are, and how a render gets at them.
//!
//! ## A file is its hash; the path is only where it sits
//!
//! A stored document names media exactly as a `.scor` folder's does — a
//! project-relative `path` and a `sha256` on each asset — so it stays a
//! document `core` reads unchanged, and **"no absolute paths, ever" stays
//! true**: nothing in a stored document knows where the server keeps anything.
//!
//! What identifies the file is the **`sha256`**, because that is what
//! identifies one in the user's library: stored once per (user, hash) (#535).
//! The `path` is only where the file appears inside the project. For a library
//! file it is [`library_path`] — `assets/<sha256>.<ext>` — so two different
//! files can never claim one path, the same file is at the same path in every
//! project, and the extension survives for whatever sniffs by it. Generated
//! media keeps the `generated/…` path it has always had.
//!
//! Resolving by hash is also the safety argument. When a stored project is
//! rendered ([`materialise`]), which file a path
//! links to is decided by (user, hash) — looked up by the caller in that
//! user's own storage — never by the path itself. A document cannot point the
//! server at another user's file, or at any file on the host, by writing a
//! path: the worst a bad path can do is be refused.
//!
//! ## `project_assets`
//!
//! `record` rewrites a project's rows from its document inside the same
//! transaction as every write, one row per distinct well-formed `sha256`. A
//! malformed hash names no file anyone could have, so it records nothing
//! rather than failing the save — `Project::validate` is what reports it.
//! The foreign key from these rows to the library's own table is #535's to
//! add alongside that table; until then the pair (user, hash) is the whole of
//! the reference.

mod materialise;

use std::collections::BTreeSet;

use scorsese_core::{ASSETS_DIR, Project, ProjectPath};

pub use materialise::{MaterialiseError, Materialised, MediaSource, materialise};

use super::ProjectError;
use crate::db::Tx;

/// Where a library file sits inside a stored project: `assets/<sha256>.<ext>`,
/// or `assets/<sha256>` for a file with no extension.
pub fn library_path(sha256: &str, extension: Option<&str>) -> ProjectPath {
    match extension.filter(|extension| !extension.is_empty()) {
        Some(extension) => ProjectPath::new(format!("{ASSETS_DIR}/{sha256}.{extension}")),
        None => ProjectPath::new(format!("{ASSETS_DIR}/{sha256}")),
    }
}

/// Every distinct well-formed `sha256` in the document's assets table.
pub fn hashes(project: &Project) -> BTreeSet<&str> {
    project
        .assets
        .iter()
        .filter_map(|asset| asset.sha256.as_deref())
        .filter(|hash| is_sha256(hash))
        .collect()
}

/// Lowercase hex, 64 digits: the form `core` writes and the table accepts.
fn is_sha256(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Rewrite project `id`'s `project_assets` rows from `project`.
///
/// The owner comes from the project row rather than from `member_id()`, so
/// this works the same in a user's scope and in the startup migration, which
/// runs in none.
pub(super) async fn record(tx: &mut Tx, id: i64, project: &Project) -> Result<(), ProjectError> {
    sqlx::query("DELETE FROM project_assets WHERE project_id = $1")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    let hashes: Vec<String> = hashes(project).into_iter().map(str::to_owned).collect();
    sqlx::query(
        "INSERT INTO project_assets (project_id, user_id, sha256)
         SELECT p.id, p.user_id, h FROM projects p, unnest($2::text[]) AS h WHERE p.id = $1",
    )
    .bind(id)
    .bind(hashes)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
