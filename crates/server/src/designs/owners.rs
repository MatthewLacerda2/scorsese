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
//!
//! ## Voice slots (#1261)
//!
//! The same shared account holds a fixed number of custom voices — 10 on
//! Starter (elevenlabs.io/pricing). ElevenLabs' own page says which voices take
//! one: "Voices saved from the Voice Library do not use custom voice slots.
//! Custom voice slots are only used for voices you create with Voice Design or
//! Voice Cloning." (elevenlabs.io/docs/eleven-creative/voices/voice-library,
//! read 2026-10-16). So a narration with a library voice, which the vendor
//! saves to the account by itself, costs no slot, and what does is exactly
//! what `designed_voices` records. Counted here, from scorsese's own rows: the
//! operator's key lacks the `user_read` permission the vendor's live count
//! needs, and nothing should depend on it.
//!
//! Two limits, both checked before a voice is kept ([`no_slot`]): a user keeps
//! at most [`KEPT_PER_USER`], so one person cannot take every slot from the
//! rest, and nobody keeps one once the account's slots are all taken — said in
//! plain words, rather than as whatever the vendor answers a full account
//! with. Counting everyone's rows is cross-user, which is why it is here.

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

/// The most voices one user keeps at once: a share of the account's slots,
/// so the first few people cannot take them all.
pub const KEPT_PER_USER: i64 = 3;

/// How many voice slots are taken: by one user, and by everyone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slots {
    /// The user's own kept voices.
    pub mine: i64,
    /// Every user's, together.
    pub everyone: i64,
}

/// The voice slots `user` and everybody take now.
pub async fn slots(pool: &PgPool, user: UserId) -> Result<Slots, sqlx::Error> {
    let mut tx = db::privileged(pool).await?;
    let (mine, everyone) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE user_id = $1), count(*) FROM designed_voices",
    )
    .bind(user.get())
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Slots { mine, everyone })
}

/// Why no voice can be kept, with `slots` taken of the account's `of`: the
/// user has their share, or the account is full. `None` when there is room.
pub fn no_slot(slots: Slots, of: usize) -> Option<String> {
    let account = i64::try_from(of).unwrap_or(i64::MAX);
    let instead = "Narrate with a voice you already keep (voice_design with list), or choose \
                   one from voices: a built-in or Voice Library voice takes no slot. Nothing \
                   was created or charged.";
    if slots.mine >= KEPT_PER_USER {
        Some(format!(
            "you already keep {} designed voices, the most one account keeps here. {instead}",
            slots.mine
        ))
    } else if slots.everyone >= account {
        Some(format!(
            "scorsese's voice account is full: all {account} of its slots for designed voices \
             are taken. {instead}"
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn taken(mine: i64, everyone: i64) -> Slots {
        Slots { mine, everyone }
    }

    #[test]
    fn room_under_both_limits() {
        assert_eq!(no_slot(taken(KEPT_PER_USER - 1, 9), 10), None);
    }

    #[test]
    fn a_user_with_their_share_keeps_no_more() {
        let why = no_slot(taken(KEPT_PER_USER, KEPT_PER_USER), 10).unwrap();
        assert!(why.contains("already keep 3"), "{why}");
        assert!(why.contains("Nothing was created"), "{why}");
    }

    #[test]
    fn a_full_account_keeps_no_more_for_anybody() {
        let why = no_slot(taken(0, 10), 10).unwrap();
        assert!(why.contains("all 10"), "{why}");
        assert_eq!(no_slot(taken(0, 9), 10), None);
    }
}
