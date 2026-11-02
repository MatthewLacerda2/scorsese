//! Whose a designed or cloned voice is, across every user (#687, #1263) —
//! both are rows of `designed_voices`, so every rule here holds for both.
//!
//! A kept voice lives in the operator's ElevenLabs account, the one every
//! user's jobs share, and its `voice_id` is only a string: a user who learns
//! another's could narrate with it, or look it up and read the description it
//! was designed from. Row-level security cannot stop that, because the voice
//! is not a row — it is the vendor's. So the rule is applied where a voice id
//! is used (`generate`) or asked about (`voices` with `check`), and it is an
//! **allow-list** (#1655): a voice in the caller's own `designed_voices` is
//! theirs; one in *another* user's is refused at once; and any other is asked
//! of the vendor (`Catalogue::origin`), which lets built-in and Voice Library
//! voices through and refuses a custom voice the account holds that nobody's
//! ledger names — one made outside the web app, or whose row is gone.
//!
//! Telling "another user's" from "nobody's" means reading past the caller's
//! own rows, so this is one of [`privileged`](crate::db::privileged)'s short
//! list. It answers which ids are whose and nothing about them.
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
//! at most their share — [`KEPT_FREE`] on an account that has never bought
//! credit (no paid `payments` row it kept), [`KEPT_BOUGHT`] once it has (#1655) — so
//! free accounts cannot take every slot from the rest; and nobody keeps one
//! once the account's slots are all taken. Both said in plain words, rather
//! than as whatever the vendor answers a full account with. A share only
//! limits adding: a user over it keeps what they have. Counting everyone's
//! rows is cross-user, which is why it is here.

use sqlx::postgres::PgPool;

use crate::db::{self, UserId};

/// Which of `voices` the ledger names, and whose.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Named {
    /// Those `user` designed or cloned and keeps.
    pub mine: Vec<String>,
    /// Those another user keeps.
    pub others: Vec<String>,
}

/// Which of `voices` `user` keeps, and which another user does.
pub async fn named(pool: &PgPool, user: UserId, voices: &[String]) -> Result<Named, sqlx::Error> {
    if voices.is_empty() {
        return Ok(Named::default());
    }
    let mut tx = db::privileged(pool).await?;
    let found: Vec<(String, bool)> = sqlx::query_as(
        "SELECT DISTINCT voice_id, user_id = $2 FROM designed_voices
         WHERE voice_id = ANY($1)
         ORDER BY voice_id",
    )
    .bind(voices)
    .bind(user.get())
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let mut named = Named::default();
    for (voice, mine) in found {
        if mine {
            named.mine.push(voice);
        } else {
            named.others.push(voice);
        }
    }
    Ok(named)
}

/// The refusal for narrating with, or asking about, `voice`: another user's.
pub fn refusal(voice: &str) -> String {
    format!(
        "voice {voice} is not one you can use: it was made by another account, and a voice \
         designed or cloned here is only its maker's. Choose a voice from voices, or make your \
         own with voice_design or voice_clone."
    )
}

/// The refusal for `voice`: a custom voice in the account that no ledger of
/// the caller's names (#1655).
pub fn not_kept(voice: &str) -> String {
    format!(
        "voice {voice} is not one you can use: it is a custom voice you did not design or \
         clone here, and a custom voice is only its maker's. Choose a voice from voices, or \
         make your own with voice_design or voice_clone."
    )
}

/// The most voices a user who has never bought credit keeps at once.
pub const KEPT_FREE: i64 = 1;

/// The most voices a user who has bought credit keeps at once: a share of
/// the account's slots, so the first few people cannot take them all.
pub const KEPT_BOUGHT: i64 = 3;

/// How many voice slots are taken: by one user, and by everyone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slots {
    /// The user's own kept voices.
    pub mine: i64,
    /// Every user's, together.
    pub everyone: i64,
    /// Whether the user has ever bought credit: a paid `payments` row they
    /// did not withdraw (`payments::withdraw`), since money given back is
    /// credit never bought.
    pub bought: bool,
}

impl Slots {
    /// How many voices the user may keep at once.
    pub fn share(&self) -> i64 {
        if self.bought { KEPT_BOUGHT } else { KEPT_FREE }
    }
}

/// The voice slots `user` and everybody take now, and the user's share.
pub async fn slots(pool: &PgPool, user: UserId) -> Result<Slots, sqlx::Error> {
    let mut tx = db::privileged(pool).await?;
    let (mine, everyone, bought) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE user_id = $1), count(*),
                EXISTS (SELECT 1 FROM payments p
                        WHERE p.user_id = $1 AND p.state = 'paid'
                          AND NOT EXISTS (SELECT 1 FROM credit_entries c
                                          WHERE c.payment_id = p.id
                                            AND c.kind = 'withdrawal'))
         FROM designed_voices",
    )
    .bind(user.get())
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Slots {
        mine,
        everyone,
        bought,
    })
}

/// Why no voice can be kept, with `slots` taken of the account's `of`: the
/// user has their share, or the account is full. `None` when there is room.
pub fn no_slot(slots: Slots, of: usize) -> Option<String> {
    let account = i64::try_from(of).unwrap_or(i64::MAX);
    let instead = "Narrate with a voice you already keep (voice_design or voice_clone with \
                   list), forget one you no longer need (voice_design or voice_clone with \
                   forget) to free its slot, or choose one from voices: a built-in or Voice \
                   Library voice takes no slot. Nothing was created or charged.";
    if !slots.bought && slots.mine >= KEPT_FREE {
        Some(format!(
            "you already keep {} {} you designed or cloned: free accounts keep one custom \
             voice; buying credit lets you keep three. {instead}",
            slots.mine,
            if slots.mine == 1 { "voice" } else { "voices" }
        ))
    } else if slots.mine >= slots.share() {
        Some(format!(
            "you already keep {} voices you designed or cloned, the most one account keeps \
             here. {instead}",
            slots.mine
        ))
    } else if slots.everyone >= account {
        Some(format!(
            "scorsese's voice account is full: all {account} of its slots for designed and \
             cloned voices are taken. {instead}"
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn taken(mine: i64, everyone: i64) -> Slots {
        Slots {
            mine,
            everyone,
            bought: true,
        }
    }

    fn free(mine: i64, everyone: i64) -> Slots {
        Slots {
            bought: false,
            ..taken(mine, everyone)
        }
    }

    #[test]
    fn room_under_both_limits() {
        assert_eq!(no_slot(taken(KEPT_BOUGHT - 1, 9), 10), None);
        assert_eq!(no_slot(free(KEPT_FREE - 1, 9), 10), None);
    }

    #[test]
    fn a_free_account_keeps_one() {
        let why = no_slot(free(KEPT_FREE, 1), 10).unwrap();
        assert!(why.contains("already keep 1 voice you"), "{why}");
        assert!(why.contains("buying credit lets you keep three"), "{why}");
        // Over its share — kept before #1655 — it adds no more, and is told why.
        let why = no_slot(free(3, 3), 10).unwrap();
        assert!(why.contains("already keep 3 voices you"), "{why}");
        assert!(why.contains("free accounts keep one"), "{why}");
    }

    #[test]
    fn a_user_with_their_share_keeps_no_more() {
        let why = no_slot(taken(KEPT_BOUGHT, KEPT_BOUGHT), 10).unwrap();
        assert!(why.contains("already keep 3"), "{why}");
        assert!(why.contains("Nothing was created"), "{why}");
        assert!(why.contains("voice_clone with"), "{why}");
    }

    #[test]
    fn a_full_account_keeps_no_more_for_anybody() {
        let why = no_slot(taken(0, 10), 10).unwrap();
        assert!(why.contains("all 10"), "{why}");
        assert_eq!(no_slot(taken(0, 9), 10), None);
    }
}
