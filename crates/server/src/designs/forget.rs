//! Forgetting a kept voice (#1381): deleting it at the vendor, which frees the
//! voice slot it takes in the operator's account, and then its row, which
//! frees the slot in [`owners::slots`](super::owners::slots)' count.
//!
//! **The vendor first, then the row.** A row whose voice is gone at the vendor
//! only over-counts until the next attempt, which finds no such voice — the
//! studio answers that as already forgotten — and drops the row. The other
//! order would leave a voice taking a slot that nothing counts any more, the
//! drift the count exists to prevent. The design stays, with its description
//! and seed, so the same candidate can be kept again.
//!
//! Two callers. The `forget` verb of `voice_design` queues [`handler`]'s job:
//! one way to reach a vendor, under the same ElevenLabs budget as every other
//! call there. Deleting an account calls [`everything`] before the rows go
//! by cascade, and a vendor failure there does not stop the deletion: what
//! could not be deleted is said, for the operator to delete by hand.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::postgres::PgPool;

use crate::db::{self, Says, Tx, UserId};
use crate::generations::Vendors;
use crate::jobs::{Context, Handler, Job, Outcome, blocking};

/// What a forget job carries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgetPayload {
    /// The kept voice to delete.
    pub voice_id: String,
}

/// The handler for [`crate::jobs::kinds::VOICE_FORGET`].
pub fn handler(vendors: Arc<dyn Vendors>) -> impl Handler {
    move |job: Job, context: Context| {
        let vendors = Arc::clone(&vendors);
        async move {
            match serde_json::from_value::<ForgetPayload>(job.payload.clone()) {
                Ok(payload) => match forget(&context, &payload.voice_id, vendors.as_ref()).await {
                    Ok(()) => Outcome::Done(json!({ "forgotten": payload.voice_id })),
                    Err(why) => Outcome::Failed(why),
                },
                Err(error) => {
                    Outcome::Failed(format!("the job does not describe a voice: {error}"))
                }
            }
        }
    }
}

async fn forget(context: &Context, voice_id: &str, vendors: &dyn Vendors) -> Result<(), String> {
    let mut tx = context.scoped().await.map_err(database)?;
    let mine = kept(&mut tx).await.map_err(database)?;
    tx.commit().await.map_err(database)?;
    if !mine.iter().any(|kept| kept == voice_id) {
        return Err(format!(
            "{voice_id} is not a voice you keep, so nothing was deleted; voice_design with \
             list shows the ones you do"
        ));
    }
    at_vendor(vendors, voice_id).await?;
    let mut tx = context.scoped().await.map_err(database)?;
    drop_row(&mut tx, voice_id).await.map_err(database)?;
    tx.commit().await.map_err(database)
}

/// Delete every voice `user` keeps at the vendor, before their account goes.
/// Answers with a line for each that could not be deleted — still in the
/// operator's ElevenLabs account, for them to delete there.
pub async fn everything(
    pool: &PgPool,
    user: UserId,
    vendors: &dyn Vendors,
) -> Result<Vec<String>, sqlx::Error> {
    let mut tx = db::scoped(pool, user).await?;
    let voices = kept(&mut tx).await?;
    tx.commit().await?;
    let mut left = Vec::new();
    for voice in voices {
        if let Err(why) = at_vendor(vendors, &voice).await {
            tracing::error!("forgetting voice {voice}: {why}");
            left.push(format!("{voice}: {why}"));
        }
    }
    Ok(left)
}

/// `voice_id` deleted at the vendor, or why not.
async fn at_vendor(vendors: &dyn Vendors, voice_id: &str) -> Result<(), String> {
    let studio = vendors.studio()?;
    let voice = voice_id.to_owned();
    blocking("forgetting the voice crashed", move || {
        studio.forget(&voice)
    })
    .await?
    .map_err(|error| format!("{error} — the voice was not deleted"))
}

/// The voice ids the scoped user keeps.
async fn kept(tx: &mut Tx) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT voice_id FROM designed_voices ORDER BY id")
        .fetch_all(&mut **tx)
        .await
}

/// The scoped user's row for `voice_id`, gone.
async fn drop_row(tx: &mut Tx, voice_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM designed_voices WHERE voice_id = $1")
        .bind(voice_id)
        .execute(&mut **tx)
        .await
        .map(drop)
}

/// A failure on the server, logged and said without its detail.
fn database(error: impl std::fmt::Display) -> String {
    db::user_facing("voice forget job", Says::Books("voice design"), error)
}
