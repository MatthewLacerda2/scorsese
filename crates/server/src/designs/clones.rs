//! A voice cloned for a web user (#1263): what `voice_clone` does locally,
//! with the user's library holding the recordings and `designed_voices`
//! (`kind = 'cloned'`, migration `0035` has why it is that table) holding
//! what `cloned-voices.json` holds in a `.scor` folder.
//!
//! **It runs as a `voice_keep` job**, not a kind of its own: a clone takes a
//! voice slot exactly as keeping a design does, and that kind runs one job at
//! a time so the slot count it checks first ([`owners::no_slot`]) cannot be
//! raced — by a keep or by another clone. The payloads tell the two apart
//! ([`Keeping`]). Free, as locally: creating a clone is not billed.

use scorsese_providers::voices::clone::{Recording, Request, media_type};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::types::Json;

use super::{KeepPayload, owners};
use crate::db::{self, Says, Tx, UserId};
use crate::generations::Vendors;
use crate::jobs::{Context, blocking};
use crate::storage::Storage;

/// One recording a clone is made from: a library item of the user's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recorded {
    /// The library item.
    pub item: i64,
    /// Its content hash, which is where its file is.
    pub sha256: String,
    /// Its extension, which with the hash is its file's name.
    pub extension: String,
    /// The name it is sent under — the item's own.
    pub name: String,
}

/// What a clone job carries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClonePayload {
    /// What to call the voice.
    pub name: String,
    /// What the voice is, kept with it at the vendor.
    pub description: Option<String>,
    /// Ask the vendor to clean background noise out first.
    pub remove_background_noise: bool,
    /// The recordings, in the order they were named.
    pub recordings: Vec<Recorded>,
}

/// What a `voice_keep` job carries: a design's candidate, or recordings.
/// Untagged, so the payloads queued before cloning existed still read.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Keeping {
    /// A candidate of a design, kept.
    Design(KeepPayload),
    /// A voice cloned from recordings.
    Clone(ClonePayload),
}

/// A voice the user cloned, as `voice_clone` with `list` reads it back.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct Cloned {
    /// The id a narration names.
    pub voice_id: String,
    /// What it was called when it was cloned.
    pub name: String,
    /// The recordings it was made from.
    pub recordings: Json<Vec<Recorded>>,
}

/// Every voice the scoped user cloned, oldest first.
pub async fn cloned(tx: &mut Tx) -> Result<Vec<Cloned>, sqlx::Error> {
    sqlx::query_as(
        "SELECT voice_id, name, recordings FROM designed_voices WHERE kind = 'cloned'
         ORDER BY id",
    )
    .fetch_all(&mut **tx)
    .await
}

/// Clone `payload`'s voice for `user`, unless they or the account have no
/// voice slot left, and record it.
pub(super) async fn clone(
    context: &Context,
    user: UserId,
    voice_slots: usize,
    storage: &Storage,
    payload: &ClonePayload,
    vendors: &dyn Vendors,
) -> Result<Value, String> {
    let slots = owners::slots(context.pool(), user)
        .await
        .map_err(database)?;
    if let Some(why) = owners::no_slot(slots, voice_slots) {
        return Err(why);
    }
    let mut recordings = Vec::with_capacity(payload.recordings.len());
    for recorded in &payload.recordings {
        let file = storage.library_file(user, &recorded.sha256, &recorded.extension);
        let bytes = tokio::fs::read(&file).await.map_err(|error| {
            tracing::error!("reading a recording to clone: {error}");
            format!(
                "the recording \"{}\" is no longer in your library — nothing was created",
                recorded.name
            )
        })?;
        let file_name = format!("{}.{}", recorded.sha256, recorded.extension);
        recordings.push(Recording {
            media_type: media_type(&file_name).to_owned(),
            file_name,
            bytes,
        });
    }
    let request = Request {
        name: payload.name.clone(),
        description: payload.description.clone(),
        recordings,
        remove_background_noise: payload.remove_background_noise,
    };
    let studio = vendors.studio()?;
    let provider = studio.name();
    let made = blocking("cloning the voice crashed", move || {
        studio.clone_voice(&request)
    })
    .await?
    .map_err(|error| format!("{error}"))?;
    let mut tx = context.scoped().await.map_err(database)?;
    let recorded = serde_json::to_value(&payload.recordings).map_err(database)?;
    sqlx::query(
        "INSERT INTO designed_voices (user_id, kind, voice_id, name, provider, recordings)
         VALUES (member_id(), 'cloned', $1, $2, $3, $4)",
    )
    .bind(&made.voice.id)
    .bind(&made.voice.name)
    .bind(provider)
    .bind(recorded)
    .execute(&mut *tx)
    .await
    .map_err(database)?;
    tx.commit().await.map_err(database)?;
    Ok(json!({
        "voice_id": made.voice.id,
        "name": made.voice.name,
        "cloned": true,
        "requires_verification": made.requires_verification,
    }))
}

/// A failure on the server, logged and said without its detail.
fn database(error: impl std::fmt::Display) -> String {
    db::user_facing("voice clone job", Says::Books("voice design"), error)
}
