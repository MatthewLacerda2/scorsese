//! Each item's thumbnail: a job, the first handler the queue runs.
//!
//! Drawn by `scorsese-render` ([`scorsese_render::frames::thumbnail`]), the
//! one place ffmpeg is invoked, into the **cache** — a thumbnail can always be
//! drawn again from the file, so it is not worth a place in the nightly
//! backup. Queued when an item is admitted, in the same transaction; queued
//! again by [`Library::thumbnail`] when one is asked for and is not there —
//! a cache that was cleared, or a job that failed — so the cache really is
//! rebuildable rather than merely described as such.
//!
//! What a thumbnail shows: a video's frame one second in (or halfway through a
//! shorter one), a picture scaled down, a sound's waveform.

use std::path::{Path, PathBuf};

use scorsese_render::Tools;
use scorsese_render::frames::{Thumbnail, thumbnail};
use serde_json::{Value, json};

use super::{Kind, Library, LibraryError, store};
use crate::db::UserId;
use crate::jobs::{Context, Handler, Job, Outcome, kinds, store as jobs};
use crate::storage::Storage;

/// Where the thumbnail of `user`'s file with this hash and kind is kept.
pub fn path(storage: &Storage, user: UserId, sha256: &str, kind: Kind) -> PathBuf {
    storage.thumbnail(user, sha256, drawing(kind, None).extension())
}

/// How a file of `kind` is drawn, given how long it lasts.
fn drawing(kind: Kind, duration: Option<f64>) -> Thumbnail {
    match kind {
        Kind::Video => Thumbnail::Frame {
            at_seconds: duration.map_or(0.0, |seconds| (seconds / 2.0).min(1.0)),
        },
        Kind::Image => Thumbnail::Picture,
        Kind::Audio => Thumbnail::Waveform,
    }
}

/// The handler for [`kinds::THUMBNAIL`]: draw the item named in the payload.
pub fn handler(storage: Storage, tools: Tools) -> impl Handler {
    move |job: Job, context: Context| {
        let (storage, tools) = (storage.clone(), tools.clone());
        async move {
            match draw(&storage, &tools, &job, &context).await {
                Ok(done) => Outcome::Done(done),
                Err(why) => Outcome::Failed(why),
            }
        }
    }
}

async fn draw(
    storage: &Storage,
    tools: &Tools,
    job: &Job,
    context: &Context,
) -> Result<Value, String> {
    let id = job.payload["item"]
        .as_i64()
        .ok_or("the job does not name an item")?;
    let item = {
        let mut tx = context.scoped().await.map_err(|error| error.to_string())?;
        let item = store::read(&mut tx, id).await;
        tx.commit().await.map_err(|error| error.to_string())?;
        match item {
            Ok(item) => item,
            // Deleted before its turn came: nothing left to draw.
            Err(LibraryError::NotFound) => return Ok(json!({ "item": id, "deleted": true })),
            Err(error) => return Err(error.to_string()),
        }
    };
    let source = storage.library_file(job.user, &item.sha256, &item.extension);
    let out = path(storage, job.user, &item.sha256, item.kind);
    let what = drawing(item.kind, item.media.duration_seconds);
    let tools = tools.clone();
    tokio::task::spawn_blocking(move || write(&tools, &source, what, &out))
        .await
        .map_err(|error| error.to_string())??;
    Ok(json!({ "item": id }))
}

/// Draw to a file beside `out` and rename it into place, so a thumbnail is
/// never read half-written.
fn write(tools: &Tools, source: &Path, what: Thumbnail, out: &Path) -> Result<(), String> {
    let drawing = out.with_extension(format!("drawing.{}", what.extension()));
    thumbnail(tools, source, what, &drawing).map_err(|error| {
        eprintln!("scorsese-server: {error}");
        "the thumbnail could not be drawn".to_owned()
    })?;
    std::fs::rename(&drawing, out).map_err(|error| error.to_string())
}

impl Library {
    /// Where `user`'s item `id`'s thumbnail is, when it has been drawn.
    ///
    /// When it has not, and no job is already drawing it, one is queued, and
    /// `None` says to ask again later. A file whose thumbnail just failed is
    /// left an hour before trying again, so a list that keeps asking does not
    /// keep a broken file's job running.
    pub async fn thumbnail(
        &self,
        user: UserId,
        id: i64,
    ) -> Result<Option<PathBuf>, LibraryError> {
        let item = self.get(user, id).await?;
        let file = path(&self.storage, user, &item.sha256, item.kind);
        if file.is_file() {
            return Ok(Some(file));
        }
        let mut tx = crate::db::scoped(&self.pool, user).await?;
        let pending: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM jobs WHERE kind = $1 AND payload->>'item' = $2::text
                AND (state IN ('waiting', 'running')
                     OR (state = 'failed' AND finished_at > now() - interval '1 hour')))",
        )
        .bind(kinds::THUMBNAIL.name)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let job = if pending {
            None
        } else {
            Some(jobs::enqueue(&mut tx, kinds::THUMBNAIL, &json!({ "item": id })).await?)
        };
        tx.commit().await?;
        if let Some(job) = job {
            self.queue.announce(user, &job);
        }
        Ok(None)
    }
}
