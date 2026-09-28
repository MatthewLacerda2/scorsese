//! Each heavy video's preview proxy (#542): a job, made by `scorsese-render`.
//!
//! A proxy is a small copy of a video that a **preview** decodes instead of
//! the original ([`scorsese_render::preview`] has what one is and why). It is
//! drawn by [`scorsese_render::preview::make`], the one place the transcode is
//! spelled, into the **cache** beside the thumbnails
//! ([`Storage::proxy`](crate::storage::Storage::proxy)), because it can always
//! be made again from the file.
//!
//! **When:** queued when a video worth one is admitted, in the same
//! transaction as its thumbnail — upload is the moment the file is new and
//! nobody is waiting on the machine for it yet, so the first preview it
//! appears in is already fast. And queued again by a preview render that finds
//! one missing ([`for_preview`]) — a cache cleared by hand, or a job that
//! failed — so the cache is rebuildable in fact and not just in name. That
//! preview reads the original meanwhile: correct, only slower.
//!
//! **Never a final render's.** Only the preview render job asks for proxies;
//! a finished render is rendered with no [`scorsese_render::Preview`] at all,
//! so it cannot read one. The render tests hold that with a proxy of another
//! colour sitting in the cache.
//!
//! Proxies are removed with their item and otherwise kept: at a quarter of
//! the short side they are a small fraction of the library they stand in for,
//! and the render quota is about renders.

use scorsese_render::Tools;
use scorsese_render::preview::{self, Proxies};
use serde_json::{Value, json};

use super::store::{self, ItemRow, item_columns};
use super::{Item, LibraryError};
use crate::db::{Tx, UserId};
use crate::jobs::{Context, Handler, Job, JobView, Outcome, kinds, store as jobs};
use crate::storage::Storage;

/// Whether `item` gets a proxy: a video larger than one, known to be opaque.
pub(super) fn worth_one(item: &Item) -> bool {
    preview::worth_making(item.kind.asset_kind(), &item.media)
}

/// The handler for [`kinds::PROXY`]: make the proxy of the item in the payload.
pub fn handler(storage: Storage, tools: Tools) -> impl Handler {
    move |job: Job, context: Context| {
        let (storage, tools) = (storage.clone(), tools.clone());
        async move {
            match make(&storage, &tools, &job, &context).await {
                Ok(done) => Outcome::Done(done),
                Err(why) => Outcome::Failed(why),
            }
        }
    }
}

async fn make(
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
            // Deleted before its turn came: nothing to stand in for.
            Err(LibraryError::NotFound) => return Ok(json!({ "item": id, "deleted": true })),
            Err(error) => return Err(error.to_string()),
        }
    };
    let out = storage.proxy(job.user, &item.sha256);
    if !worth_one(&item) || out.is_file() {
        return Ok(json!({ "item": id, "made": false }));
    }
    let source = storage.library_file(job.user, &item.sha256, &item.extension);
    let tools = tools.clone();
    tokio::task::spawn_blocking(move || preview::make(&tools, &source, &out))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| {
            // ffmpeg's words name the server's paths: for the log.
            eprintln!("scorsese-server: {error}");
            "the preview proxy could not be made".to_owned()
        })?;
    Ok(json!({ "item": id, "made": true }))
}

/// The proxies a preview of a project using these files may read, and the
/// proxy jobs this queued for the ones worth having that are not made.
///
/// Scoped: `tx` finds only its user's items, so a document naming another
/// user's hash gets no proxy for it (and the materialiser no file). A file
/// whose proxy failed in the last hour is not retried, so a preview asked
/// for again and again does not keep a broken file's transcode running.
pub(crate) async fn for_preview(
    tx: &mut Tx,
    storage: &Storage,
    user: UserId,
    hashes: &[String],
) -> Result<(Proxies, Vec<JobView>), sqlx::Error> {
    let rows: Vec<ItemRow> = sqlx::query_as(concat!(
        "SELECT ",
        item_columns!(),
        " FROM library_items WHERE sha256 = ANY($1)"
    ))
    .bind(hashes)
    .fetch_all(&mut **tx)
    .await?;
    let (mut proxies, mut queued) = (Proxies::new(), Vec::new());
    for item in rows.into_iter().filter_map(|row| store::item(row).ok()) {
        let file = storage.proxy(user, &item.sha256);
        if file.is_file() {
            proxies.insert(item.sha256, file);
        } else if worth_one(&item) && !pending(tx, item.id).await? {
            let payload = json!({ "item": item.id });
            queued.push(jobs::enqueue(tx, kinds::PROXY, &payload).await?);
        }
    }
    Ok((proxies, queued))
}

/// Whether item `id` has a proxy job on its way, or one that failed lately.
async fn pending(tx: &mut Tx, id: i64) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM jobs WHERE kind = $1 AND payload->>'item' = $2::text
            AND (state IN ('waiting', 'running')
                 OR (state = 'failed' AND finished_at > now() - interval '1 hour')))",
    )
    .bind(kinds::PROXY.name)
    .bind(id)
    .fetch_one(&mut **tx)
    .await
}
