//! Whose a designed voice is, across every user (#687).
//!
//! A kept voice lives in the operator's ElevenLabs account, the one every
//! user's jobs share, and its `voice_id` is only a string: a user who learns
//! another's could narrate with it, or look it up and read the description it
//! was designed from. Row-level security cannot stop that, because the voice
//! is not a row — it is the vendor's. So the rule is applied where a voice id
//! is used (`generate`) or asked about (`voices` with `check`): a voice in
//! *another* user's `designed_voices` is refused. Built-in and Voice Library
//! voices are in nobody's ledger, so they stay open to everyone, and a user's
//! own designed voices are theirs to use.
//!
//! Telling "another user's" from "nobody's" means reading past the caller's
//! own rows, so this is one of [`privileged`](crate::db::privileged)'s short
//! list. It answers which ids are someone else's and nothing about them.

use sqlx::postgres::PgPool;

use crate::db::{self, UserId};

/// Which of `voices` a user other than `user` designed and kept.
pub async fn others(
    pool: &PgPool,
    user: UserId,
    voices: &[String],
) -> Result<Vec<String>, sqlx::Error> {
    if voices.is_empty() {
        return Ok(Vec::new());
    }
    let mut tx = db::privileged(pool).await?;
    let found = sqlx::query_scalar(
        "SELECT DISTINCT voice_id FROM designed_voices
         WHERE voice_id = ANY($1) AND user_id <> $2
         ORDER BY voice_id",
    )
    .bind(voices)
    .bind(user.get())
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(found)
}

/// The refusal for narrating with, or asking about, `voice`: another user's.
pub fn refusal(voice: &str) -> String {
    format!(
        "voice {voice} is not one you can use: it was designed by another account, and a \
         designed voice is only its maker's. Choose a voice from voices, or design your own \
         with voice_design."
    )
}
