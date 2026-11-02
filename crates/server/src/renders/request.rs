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
use crate::credits::ledger;
use crate::db::{self, UserId};
use crate::http::limits::Limits;
use crate::jobs::{JobView, Queue, kinds, store as jobs};
use crate::library::{LibraryError, room};
use crate::projects::{self, ProjectError};
use crate::storage::Storage;

/// The most renders and previews one member may have waiting at once
/// (#1648), every project together. A preview queued cancels the project's
/// waiting previews of earlier revisions at its settings first, so an edit's
/// previews never fill the queue.
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
    /// Too many renders and previews were queued this minute: the seconds
    /// until one more may be.
    #[error("too many renders and previews asked for this minute; try again in {0} seconds")]
    TooMany(u64),
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
/// kept, on its way, or queued now and announced on `queue`. A new job is
/// counted against `asks` when given: asking again about one kept or on its
/// way, as the editor's poll does, is never refused.
pub async fn ask(
    pool: &PgPool,
    queue: &Queue,
    cache: &RenderCache,
    storage: &Storage,
    asks: Option<&Limits>,
    (user, id): (UserId, i64),
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
    // Counted under the user's lock, so asks at once cannot each see room
    // for one more.
    ledger::lock(&mut tx).await?;
    let (retired, outrun) = if settings.preview.is_some() {
        (
            store::retire_waiting(&mut tx, id, &settings, &key).await?,
            store::outrun(&mut tx, id, &settings, &key).await?,
        )
    } else {
        (Vec::new(), Vec::new())
    };
    let waiting: i64 =
        sqlx::query_scalar("SELECT count(*) FROM jobs WHERE state = 'waiting' AND kind = ANY($1)")
            .bind([kinds::RENDER.name, kinds::PREVIEW.name])
            .fetch_one(&mut *tx)
            .await?;
    // Refused, the transaction rolls back, and what it retired waits on.
    if waiting >= MOST_WAITING {
        return Err(AskError::Crowded);
    }
    if let Some(asks) = asks {
        asks.take(user, 1)
            .map_err(|wait| AskError::TooMany(wait.as_secs().max(1)))?;
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
    for view in retired {
        queue.tell(user, view);
    }
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
