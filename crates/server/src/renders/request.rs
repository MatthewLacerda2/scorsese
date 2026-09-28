//! Asking for a render: the one decision, shared by the route and the tool.
//!
//! `POST /api/projects/{id}/renders` and web MCP's `render` (#539) ask the
//! same question — and so does `POST /api/projects/{id}/previews` (#542), with
//! settings marked as a preview — *the project as it is now, in this shape: is it kept, on
//! its way, or to be made?* — and must answer it the same way, so it is
//! answered here once and each of them only says the answer in its own words.

use sqlx::postgres::PgPool;

use super::job::Payload;
use super::{RenderCache, RenderView, Settings, key, store};
use crate::db::{self, UserId};
use crate::jobs::{JobView, Queue, kinds, store as jobs};
use crate::projects::{self, ProjectError};

/// Where the render asked for is.
#[derive(Debug, Clone, PartialEq)]
pub enum Asked {
    /// Already made: download it now.
    Kept(RenderView),
    /// On its way — queued by this ask, or by an earlier one still running.
    Queued(JobView),
}

/// Why a render could not be asked for.
#[derive(Debug, thiserror::Error)]
pub enum AskError {
    /// The project does not render in that shape, and the message says why.
    #[error("{0}")]
    Invalid(String),
    /// The project is not there, or does not read.
    #[error(transparent)]
    Project(#[from] ProjectError),
    /// The database refused.
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
}

/// The render of `user`'s project `id` as it is now, in `settings`' shape —
/// kept, on its way, or queued now and announced on `queue`.
pub async fn ask(
    pool: &PgPool,
    queue: &Queue,
    cache: &RenderCache,
    user: UserId,
    id: i64,
    settings: Settings,
) -> Result<Asked, AskError> {
    let stored = projects::open(pool, user, id).await?;
    let key = key(&stored.document, &settings).map_err(|e| AskError::Invalid(e.to_string()))?;

    let mut tx = db::scoped(pool, user).await?;
    if let Some((view, path)) = store::take(&mut tx, id, &key).await? {
        if cache.absolute(std::path::Path::new(&path)).is_file() {
            tx.commit().await?;
            return Ok(Asked::Kept(view));
        }
        // The file went — a cache cleared by hand — so the row promises
        // nothing, and the render is made again.
        store::forget(&mut tx, view.id).await?;
    }
    // A preview queues as its own kind, so previews never hold a slot a
    // finished render is waiting for (`jobs::kinds::PREVIEW`).
    let kind = if settings.preview.is_some() {
        kinds::PREVIEW
    } else {
        kinds::RENDER
    };
    if let Some(job) = store::pending(&mut tx, kind, id, &key).await? {
        tx.commit().await?;
        return Ok(Asked::Queued(job));
    }
    let payload = Payload {
        project: id,
        key,
        settings,
        document: serde_json::to_value(&stored.document)
            .map_err(|e| AskError::Invalid(e.to_string()))?,
    };
    let payload = serde_json::to_value(&payload).map_err(|e| AskError::Invalid(e.to_string()))?;
    let job = jobs::enqueue(&mut tx, kind, &payload).await?;
    tx.commit().await?;
    queue.announce(user, &job);
    Ok(Asked::Queued(job))
}
