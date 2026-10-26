//! A library recording's speech, heard by the Whisper container (#1549): a
//! job, and the words kept on the item's row.
//!
//! Locally a transcript is a file beside the asset
//! ([`scorsese_core::words`]). A library item is one file, so here the words
//! ride on its row instead — the `words` column a spoken line's timings
//! already use (migration `0024`) — and are written out beside the asset
//! whenever a project is laid out as a folder
//! ([`crate::projects::media::materialise`]): captions, pages and
//! `project_describe` read them as they do a line's. One file is one
//! transcript in every project that uses it, corrections included.
//!
//! **Free, and a job**, as a render is: it costs the machine minutes, so it
//! queues ([`kinds::TRANSCRIBE`], one at a time) and its owner can stop it.
//! A stopped job's answer, when the server gives it anyway, is thrown away.
//!
//! **What it executes stays offline.** The recording goes to the Whisper
//! container over a network with no way out (`deploy/compose.yaml`); the
//! decode is `scorsese_render`'s, here, as a thumbnail's is.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use scorsese_core::words::Words;
use scorsese_providers::transcribe::Language;
use scorsese_render::Tools;
use scorsese_render::audio::speech;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{Library, LibraryError, store};
use crate::db::UserId;
use crate::generations::Vendors;
use crate::jobs::{Context, Handler, Job, JobView, Outcome, kinds, store as jobs};
use crate::storage::Storage;

/// How often a running job looks for its owner's stop.
const LISTEN_FOR_STOP: Duration = Duration::from_millis(250);

/// What a transcription job carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Payload {
    /// The library item to hear.
    pub item: i64,
    /// The language forced, as a code; detected when `None`.
    pub language: Option<String>,
}

/// An item's transcript, if it has one.
#[derive(Debug, Clone, PartialEq)]
pub struct Held {
    /// The item.
    pub item: i64,
    /// Its words, when it has been transcribed.
    pub words: Option<Words>,
}

impl Library {
    /// `user`'s item with this hash, and its transcript.
    pub async fn transcript(&self, user: UserId, sha256: &str) -> Result<Held, LibraryError> {
        let mut tx = crate::db::scoped(&self.pool, user).await?;
        let row: Option<(i64, Option<String>)> =
            sqlx::query_as("SELECT id, words::text FROM library_items WHERE sha256 = $1")
                .bind(sha256)
                .fetch_optional(&mut *tx)
                .await?;
        tx.commit().await?;
        let (item, words) = row.ok_or(LibraryError::NotFound)?;
        Ok(Held {
            item,
            words: words.and_then(|text| serde_json::from_str(&text).ok()),
        })
    }

    /// Keeps `words` as `user`'s item `item`'s transcript.
    pub async fn keep_transcript(
        &self,
        user: UserId,
        item: i64,
        words: &Words,
    ) -> Result<(), LibraryError> {
        let mut tx = crate::db::scoped(&self.pool, user).await?;
        keep(&mut tx, item, words).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Queues the hearing of `user`'s item, unless one is already waiting or
    /// running — which is the job handed back.
    pub async fn transcribe(
        &self,
        user: UserId,
        payload: &Payload,
    ) -> Result<JobView, LibraryError> {
        let mut tx = crate::db::scoped(&self.pool, user).await?;
        let pending: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM jobs WHERE kind = $1 AND payload->>'item' = $2::text
                AND state IN ('waiting', 'running') ORDER BY id LIMIT 1",
        )
        .bind(kinds::TRANSCRIBE.name)
        .bind(payload.item)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(id) = pending {
            tx.commit().await?;
            return jobs::get(&self.pool, user, id)
                .await?
                .ok_or(LibraryError::NotFound);
        }
        let payload =
            serde_json::to_value(payload).map_err(|e| LibraryError::Invalid(e.to_string()))?;
        let job = jobs::enqueue(&mut tx, kinds::TRANSCRIBE, &payload).await?;
        tx.commit().await?;
        self.queue.announce(user, &job);
        Ok(job)
    }
}

async fn keep(tx: &mut crate::db::Tx, item: i64, words: &Words) -> Result<(), LibraryError> {
    let kept = sqlx::query("UPDATE library_items SET words = $2::jsonb WHERE id = $1")
        .bind(item)
        .bind(words.to_json())
        .execute(&mut **tx)
        .await?;
    if kept.rows_affected() == 0 {
        return Err(LibraryError::NotFound);
    }
    Ok(())
}

/// The handler for [`kinds::TRANSCRIBE`]: hear the payload's item with the
/// Whisper `vendors` names, and keep its words on its row.
pub fn handler(storage: Storage, tools: Tools, vendors: Arc<dyn Vendors>) -> impl Handler {
    move |job: Job, context: Context| {
        let (storage, tools, vendors) = (storage.clone(), tools.clone(), Arc::clone(&vendors));
        async move {
            match hear(&storage, &tools, vendors.as_ref(), &job, &context).await {
                Ok(done) => Outcome::Done(done),
                Err(_) if context.cancel().is_cancelled() => {
                    Outcome::Cancelled("stopped before its words were kept".to_owned())
                }
                Err(why) => Outcome::Failed(why),
            }
        }
    }
}

async fn hear(
    storage: &Storage,
    tools: &Tools,
    vendors: &dyn Vendors,
    job: &Job,
    context: &Context,
) -> Result<Value, String> {
    let payload: Payload = serde_json::from_value(job.payload.clone())
        .map_err(|error| format!("the job does not read: {error}"))?;
    let language = payload
        .language
        .as_deref()
        .map(str::parse::<Language>)
        .transpose()
        .map_err(|error| error.to_string())?;
    let item = {
        let mut tx = context.scoped().await.map_err(|error| error.to_string())?;
        let item = store::read(&mut tx, payload.item).await;
        tx.commit().await.map_err(|error| error.to_string())?;
        match item {
            Ok(item) => item,
            Err(LibraryError::NotFound) => {
                return Ok(json!({ "item": payload.item, "deleted": true }));
            }
            Err(error) => return Err(error.to_string()),
        }
    };
    let ear = vendors.whisper()?;
    let source = storage.library_file(job.user, &item.sha256, &item.extension);
    let tools = tools.clone();
    let mut heard = tokio::task::spawn_blocking(move || {
        let wav = decode(&tools, &source)?;
        ear.transcribe(wav, language.as_ref())
            .map_err(|error| error.to_string())
    });
    let heard = loop {
        tokio::select! {
            heard = &mut heard => break heard.map_err(|error| error.to_string())??,
            () = tokio::time::sleep(LISTEN_FOR_STOP) => {
                if context.cancel().is_cancelled() {
                    return Err("stopped".to_owned());
                }
            }
        }
    };
    let mut tx = context.scoped().await.map_err(|error| error.to_string())?;
    keep(&mut tx, item.id, &heard.words)
        .await
        .map_err(|error| error.to_string())?;
    tx.commit().await.map_err(|error| error.to_string())?;
    Ok(json!({
        "item": item.id,
        "words": heard.words.words.len(),
        "language": heard.language,
        "seconds": heard.duration,
    }))
}

fn decode(tools: &Tools, file: &Path) -> Result<Vec<u8>, String> {
    speech(tools, file).map_err(|error| match error {
        scorsese_render::RenderError::NoSound { .. } => "the file has no sound in it".to_owned(),
        _ => "the file's sound could not be read".to_owned(),
    })
}
