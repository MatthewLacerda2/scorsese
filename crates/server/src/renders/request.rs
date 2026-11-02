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
use crate::library::{LibraryError, room};
use crate::projects::{self, ProjectError};
use crate::storage::Storage;

/// The most renders and previews one member may have waiting at once
/// (#1648), every project together. A preview of the project at the shape
/// being asked for is not counted: a new revision's preview makes it
/// worthless, and it retires when its turn comes.
pub const MOST_WAITING: i64 = 10;

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
    /// The member already has [`MOST_WAITING`] renders and previews waiting.
    #[error(
        "you already have {MOST_WAITING} renders and previews waiting, the most one may; \
         wait for one to start, or cancel one"
    )]
    Crowded,
    /// The project is not there, or does not read.
    #[error(transparent)]
    Project(#[from] ProjectError),
    /// The member has no room left, or the disk none to spare.
    #[error(transparent)]
    Storage(#[from] LibraryError),
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
    storage: &Storage,
    user: UserId,
    id: i64,
    settings: Settings,
) -> Result<Asked, AskError> {
    let stored = projects::open(pool, user, id).await?;
    // Asked of the document before a job is queued for it, so a project that
    // does not validate is told so at once; the job asks again.
    stored
        .document
        .validate()
        .map_err(|problems| AskError::Invalid(format!("cannot render this project: {problems}")))?;
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
        // Asked again while it runs — the preview's poll: say how far it is.
        return Ok(Asked::Queued(queue.progressed(job)));
    }
    // A member with no room left is refused before anything is queued; what
    // the render takes is checked again as it is kept (#1647).
    room::headroom(storage).await?;
    let used = room::kept(&mut tx, storage, user, None).await?;
    let quota = storage.quota().get();
    if used >= quota {
        return Err(LibraryError::Full {
            used,
            quota,
            needed: 0,
        }
        .into());
    }
    let outrun = if settings.preview.is_some() {
        store::outrun(&mut tx, id, &settings, &key).await?
    } else {
        Vec::new()
    };
    let waiting: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM jobs WHERE state = 'waiting' AND kind = ANY($1)
           AND NOT (kind = $2 AND payload ->> 'project' = $3::text AND payload -> 'settings' = $4)",
    )
    .bind([kinds::RENDER.name, kinds::PREVIEW.name])
    .bind(kinds::PREVIEW.name)
    .bind(id)
    .bind(sqlx::types::Json(&settings))
    .fetch_one(&mut *tx)
    .await?;
    if waiting >= MOST_WAITING {
        return Err(AskError::Crowded);
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
    // A preview still drawing an earlier revision is stopped (#660): the
    // editor is waiting for this one, and previews run one at a time.
    for running in outrun {
        if let Err(error) = crate::jobs::cancel(pool, queue, user, running).await {
            tracing::error!("could not stop preview {running}: {error}");
        }
    }
    Ok(Asked::Queued(job))
}
