//! A stored project laid out as a `.scor` folder, for as long as something
//! runs on it.
//!
//! `projects::media::materialise` does the laying out — the document written,
//! each file it names linked by hash from where the user's storage keeps it.
//! This adds the two things only a caller acting for a user knows: **which
//! files are theirs** (their own `library_items`, read scoped, so a document
//! naming somebody else's hash finds nothing), and **which briefs they have
//! already paid for**. Each prompted asset's current brief is gathered in the
//! folder, and when the library holds a file generated from that brief hash,
//! the file is linked where the brief would land — `generated/<asset>-<hash>`.
//! `scorsese_providers` then finds it there exactly as it finds a local
//! project's `generated/`: already generated, nothing to pay. That is how a
//! shot made for one project is free in another of the same user's, and never
//! in anybody else's.

use std::collections::HashMap;
use std::path::Path;

use scorsese_core::{AssetKind, Project, ProjectPath};
use scorsese_providers::{image, speech, video};
use sqlx::postgres::PgPool;

use crate::db::{self, UserId};
use crate::library::locate;
use crate::projects::media::{Materialised, hashes, materialise};
use crate::storage::Storage;

/// A project laid out as a folder, removed when this is dropped.
#[derive(Debug)]
pub struct Folder {
    laid: Materialised,
}

impl Folder {
    /// The folder: what to hand a tool as its project.
    pub fn root(&self) -> &Path {
        self.laid.root()
    }
}

/// Lay `user`'s `project` out in a scratch folder under `storage`, every file
/// it names and every generation of its current briefs linked from their
/// library.
pub async fn lay_out(
    pool: &PgPool,
    storage: &Storage,
    user: UserId,
    project: &Project,
) -> Result<Folder, String> {
    let named: Vec<String> = hashes(project).into_iter().map(str::to_owned).collect();
    let mut tx = db::scoped(pool, user).await.map_err(failed)?;
    let files = locate::by_hash(&mut tx, storage, user, &named)
        .await
        .map_err(failed)?;
    tx.commit().await.map_err(failed)?;

    let (at, document) = (storage.scratch(user), project.clone());
    let (laid, briefs) = tokio::task::spawn_blocking(move || {
        let laid = materialise(&document, &at, &|hash: &str| files.get(hash).cloned())
            .map_err(|error| format!("laying the project out: {error}"))?;
        let briefs = briefs(&document, laid.root());
        Ok::<_, String>((laid, briefs))
    })
    .await
    .map_err(|_| "laying the project out crashed; that is a bug".to_owned())??;

    let digests: Vec<String> = briefs.keys().cloned().collect();
    let mut tx = db::scoped(pool, user).await.map_err(failed)?;
    let made = locate::by_brief(&mut tx, storage, user, &digests)
        .await
        .map_err(failed)?;
    tx.commit().await.map_err(failed)?;

    let root = laid.root().to_path_buf();
    for (digest, source) in made {
        if let Some(output) = briefs.get(&digest) {
            link(&source, &output.resolve(&root))?;
        }
    }
    Ok(Folder { laid })
}

/// Every prompted asset's current brief hash, and where its output lands.
///
/// Gathered in the folder, because a shot's brief hashes the bytes of the
/// stills it names, and those are linked there. A brief that cannot be
/// gathered — no prompt yet, a still missing — has no output to find.
fn briefs(project: &Project, root: &Path) -> HashMap<String, ProjectPath> {
    let mut briefs = HashMap::new();
    for asset in &project.assets {
        match asset.kind {
            AssetKind::GeneratedVideo => {
                if let Ok(brief) = video::Brief::of(project, root, asset) {
                    briefs.insert(brief.digest(), brief.output());
                }
            }
            AssetKind::GeneratedImage => {
                if let Ok(brief) = image::Brief::of(project, root, asset) {
                    briefs.insert(brief.digest(), brief.output());
                }
            }
            AssetKind::GeneratedAudio => {
                if let Ok(brief) = speech::Brief::of(asset) {
                    briefs.insert(brief.digest(), brief.output());
                }
            }
            _ => {}
        }
    }
    briefs
}

/// Link `source` at `target`, unless something is there already — the
/// asset's own recorded file, which is the same bytes.
fn link(source: &Path, target: &Path) -> Result<(), String> {
    if target.symlink_metadata().is_ok() {
        return Ok(());
    }
    let made = target
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| symlink(source, target));
    made.map_err(|error| {
        eprintln!("scorsese-server: linking a generation into a folder: {error}");
        "laying the project out failed on the server".to_owned()
    })
}

#[cfg(unix)]
fn symlink(source: &Path, target: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source, target)
}

/// The server does not run on Windows; a hard link keeps this compiling there.
#[cfg(not(unix))]
fn symlink(source: &Path, target: &Path) -> std::io::Result<()> {
    std::fs::hard_link(source, target)
}

/// A database failure while laying out, said without its detail.
fn failed(error: sqlx::Error) -> String {
    super::database(error)
}
