//! What a preview render does that a finished render does not (#542).
//!
//! A preview is a render — the same job, the same cache, the same key rule —
//! with its settings marked by a quality ([`super::Settings::preview`]). The
//! three differences are all here, so the render job reads as one path with
//! three questions in it:
//!
//! - **Superseded before it starts.** Edits arrive faster than renders
//!   finish, and a preview of a revision the project has moved past is a
//!   picture of a document that no longer exists. So when a preview's turn
//!   comes, it first checks the project still hashes to its key, and if not it
//!   finishes at once without rendering. Nothing cancels a waiting job; it
//!   retires itself, which needs no new job state and no race with the claim.
//!   One already running finishes — at a reduced quality from proxies, that is
//!   seconds — and the editor asks again for the revision it is on.
//! - **Proxies.** A reduced quality reads each heavy video's proxy where one is
//!   made, and queues the making of any that are missing
//!   ([`crate::library::proxy`]).
//! - **One kept per shape.** A new preview replaces the project's earlier ones
//!   at the same settings: they are of documents the project has moved past.
//!   A file somebody has open is spared, exactly as eviction spares it.

use std::path::Path;

use scorsese_core::Project;
use scorsese_render::{Preview, Proxies, Quality};

use super::job::Payload;
use super::{RenderCache, Settings, key};
use crate::db::{Tx, UserId};
use crate::jobs::Context;
use crate::library::proxy;
use crate::projects;
use crate::storage::Storage;

/// Whether the project has moved on from the document this preview was asked
/// for — or is gone, which is the same answer: nobody is waiting for it.
pub(super) async fn superseded(context: &Context, user: UserId, payload: &Payload) -> bool {
    match projects::open(context.pool(), user, payload.project).await {
        Ok(stored) => key(&stored.document, &payload.settings).ok() != Some(payload.key.clone()),
        Err(_) => true,
    }
}

/// The preview to render `project` as at `quality`: its proxies, where the
/// quality reads them, with any that are missing queued and announced.
pub(super) async fn preview(
    context: &Context,
    storage: &Storage,
    user: UserId,
    project: &Project,
    quality: Quality,
) -> Result<Preview, sqlx::Error> {
    if !quality.uses_proxies() {
        return Ok(Preview::new(quality));
    }
    let hashes: Vec<String> = projects::media::hashes(project)
        .into_iter()
        .map(str::to_owned)
        .collect();
    let mut tx = context.scoped().await?;
    let (proxies, queued): (Proxies, _) =
        proxy::for_preview(&mut tx, storage, user, &hashes).await?;
    tx.commit().await?;
    for job in &queued {
        context.queue().announce(user, job);
    }
    Ok(Preview::new(quality).with_proxies(proxies))
}

/// Forget `project`'s other previews at `settings` — the same shape at the
/// same quality, so of other documents — except any file somebody has open.
/// Hands back their files, to remove once the transaction commits.
pub(super) async fn replaced(
    tx: &mut Tx,
    cache: &RenderCache,
    project: i64,
    settings: &Settings,
    kept: &str,
) -> Result<Vec<std::path::PathBuf>, sqlx::Error> {
    let pinned: Vec<String> = cache
        .pinned_paths()
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    let gone: Vec<String> = sqlx::query_scalar(
        "DELETE FROM renders
         WHERE project_id = $1 AND settings = $2 AND key <> $3 AND NOT (path = ANY($4))
         RETURNING path",
    )
    .bind(project)
    .bind(sqlx::types::Json(settings))
    .bind(kept)
    .bind(&pinned)
    .fetch_all(&mut **tx)
    .await?;
    Ok(gone
        .iter()
        .map(|path| cache.absolute(Path::new(path)))
        .collect())
}
