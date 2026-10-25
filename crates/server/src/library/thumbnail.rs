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
//! shorter one), a picture scaled down, a sound's waveform. A page or a
//! document has none ([`LibraryError::NoThumbnail`]): it is text, and drawing
//! one would mean capturing it.
//!
//! A sound's job also reads its **peaks** ([`scorsese_render::audio::peaks`],
//! #1572), the levels the timeline draws a clip's waveform from, into the
//! cache beside its picture; [`Library::peaks`] asks for them as
//! [`Library::thumbnail`] asks for a picture, and a missing one queues the
//! same job.

use std::path::{Path, PathBuf};

use scorsese_render::Tools;
use scorsese_render::audio::peaks;
use scorsese_render::frames::{Thumbnail, thumbnail};
use serde_json::{Value, json};

use super::{Kind, Library, LibraryError, store};
use crate::db::UserId;
use crate::jobs::{Context, Handler, Job, Outcome, kinds, store as jobs};
use crate::storage::Storage;

/// Where the thumbnail of `user`'s file with this hash and kind is kept —
/// `None` for a kind that has none.
pub fn path(storage: &Storage, user: UserId, sha256: &str, kind: Kind) -> Option<PathBuf> {
    let extension = drawing(kind, None)?.extension();
    Some(storage.thumbnail(user, sha256, extension))
}

/// How a file of `kind` is drawn, given how long it lasts — `None` for a page
/// or a document.
fn drawing(kind: Kind, duration: Option<f64>) -> Option<Thumbnail> {
    Some(match kind {
        Kind::Video => Thumbnail::Frame {
            at_seconds: duration.map_or(0.0, |seconds| (seconds / 2.0).min(1.0)),
        },
        Kind::Image => Thumbnail::Picture,
        Kind::Audio => Thumbnail::Waveform,
        Kind::Html | Kind::Text => return None,
    })
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
    let (Some(out), Some(what)) = (
        path(storage, job.user, &item.sha256, item.kind),
        drawing(item.kind, item.media.duration_seconds),
    ) else {
        return Ok(json!({ "item": id, "drawn": false }));
    };
    let source = storage.library_file(job.user, &item.sha256, &item.extension);
    let levels = (item.kind == Kind::Audio).then(|| storage.peaks(job.user, &item.sha256));
    let tools = tools.clone();
    let ours = (job.id, job.attempts);
    tokio::task::spawn_blocking(move || {
        write(&tools, &source, what, &out, ours)?;
        levels.map_or(Ok(()), |levels| read_peaks(&tools, &source, &levels, ours))
    })
    .await
    .map_err(|error| error.to_string())??;
    Ok(json!({ "item": id }))
}

/// The file beside `out` that job `ours` (its id and attempt) writes before
/// renaming it into place, ending in `suffix`. Two jobs can draw one item at
/// once — `/thumbnail` and `/peaks` each queue one — so each writes its own,
/// and neither renames the other's half-written file into place.
fn beside(out: &Path, ours: (i64, i32), suffix: &str) -> PathBuf {
    let (id, attempt) = ours;
    out.with_extension(format!("{id}-{attempt}.{suffix}"))
}

/// Read a sound's peaks into `out`, by way of a file beside it as a picture is.
fn read_peaks(tools: &Tools, source: &Path, out: &Path, ours: (i64, i32)) -> Result<(), String> {
    let read = peaks(tools, source).map_err(|error| {
        eprintln!("scorsese-server: {error}");
        "the sound's peaks could not be read".to_owned()
    })?;
    let reading = beside(out, ours, "reading");
    if let Some(directory) = out.parent() {
        std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    }
    std::fs::write(&reading, read.to_bytes()).map_err(|error| error.to_string())?;
    std::fs::rename(&reading, out).map_err(|error| error.to_string())
}

/// Draw to a file beside `out` and rename it into place, so a thumbnail is
/// never read half-written.
fn write(
    tools: &Tools,
    source: &Path,
    what: Thumbnail,
    out: &Path,
    ours: (i64, i32),
) -> Result<(), String> {
    let drawing = beside(out, ours, &format!("drawing.{}", what.extension()));
    thumbnail(tools, source, what, &drawing).map_err(|error| {
        eprintln!("scorsese-server: {error}");
        "the thumbnail could not be drawn".to_owned()
    })?;
    std::fs::rename(&drawing, out).map_err(|error| error.to_string())
}

impl Library {
    /// Where `user`'s item `id`'s thumbnail is, when it has been drawn;
    /// [`LibraryError::NoThumbnail`] for a kind that never has one.
    ///
    /// When it has not, and no job is already drawing it, one is queued, and
    /// `None` says to ask again later. A file whose thumbnail just failed is
    /// left an hour before trying again, so a list that keeps asking does not
    /// keep a broken file's job running.
    pub async fn thumbnail(&self, user: UserId, id: i64) -> Result<Option<PathBuf>, LibraryError> {
        let item = self.get(user, id).await?;
        let file =
            path(&self.storage, user, &item.sha256, item.kind).ok_or(LibraryError::NoThumbnail)?;
        self.drawn_or_queued(user, id, file, 1).await
    }

    /// Where `user`'s sound `id`'s peaks are, when they have been read;
    /// [`LibraryError::NoPeaks`] for anything but a sound. Otherwise as
    /// [`Library::thumbnail`]: the job that draws a sound's picture reads
    /// them, and is queued when they are missing.
    pub async fn peaks(&self, user: UserId, id: i64) -> Result<Option<PathBuf>, LibraryError> {
        let item = self.get(user, id).await?;
        if item.kind != Kind::Audio {
            return Err(LibraryError::NoPeaks);
        }
        let file = self.storage.peaks(user, &item.sha256);
        // Shorter than the rate it starts with is no peaks at all: read again,
        // never served to be kept for a year.
        self.drawn_or_queued(user, id, file, 4).await
    }

    /// `file` when it is there and holds at least `least` bytes; otherwise the drawing job for item `id` queued
    /// unless one is pending, and `None`.
    async fn drawn_or_queued(
        &self,
        user: UserId,
        id: i64,
        file: PathBuf,
        least: u64,
    ) -> Result<Option<PathBuf>, LibraryError> {
        if file
            .metadata()
            .is_ok_and(|kept| kept.is_file() && kept.len() >= least)
        {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_jobs_never_write_through_the_same_file() {
        let out = Path::new("/cache/peaks/abc.peaks");
        let first = beside(out, (7, 1), "reading");
        assert_eq!(first, Path::new("/cache/peaks/abc.7-1.reading"));
        assert_ne!(first, beside(out, (8, 1), "reading"));
        assert_ne!(first, beside(out, (7, 2), "reading"));
        assert_eq!(
            beside(Path::new("/t/abc.png"), (7, 1), "drawing.png"),
            Path::new("/t/abc.7-1.drawing.png"),
            "the picture keeps its extension last, for ffmpeg to read"
        );
    }
}
