//! Writing to the ledger, and reading a balance off it.
//!
//! Every function takes the caller's transaction, scoped to the user it acts
//! for (`db::scoped`), so an entry and whatever it belongs to — a generation
//! row, a job — are committed together or not at all. Owners are written as
//! `member_id()`, never passed in.

use serde_json::json;

pub use super::calls::{AssistantCall, Charged, charge_assistant};
pub use super::debt::owed;
use super::debt::owed_at;
use super::{CreditError, allowance};
use crate::db::Tx;

/// Which money may pay for a spend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Funds {
    /// The free allowance first, then the paid balance.
    Any,
    /// The paid balance only: a Veo shot (#1530).
    PaidOnly,
}

impl Funds {
    /// What may pay for what `link` is about: a Veo shot is paid only from
    /// the paid balance, everything else from the allowance first.
    pub fn for_link(link: &Link) -> Self {
        if link.veo.is_some() {
            Self::PaidOnly
        } else {
            Self::Any
        }
    }
}

/// A reserved amount waiting on a provider's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reservation {
    /// The reservation's entry.
    pub entry: i64,
    /// What it holds, in micro-dollars: positive, though the entry is negative.
    pub micros: i64,
}

/// What an entry is about, beyond its amount.
#[derive(Debug, Clone, Copy, Default)]
pub struct Link {
    /// The project it was spent on.
    pub project: Option<i64>,
    /// The Veo shot it paid for.
    pub veo: Option<i64>,
    /// The spoken line it paid for.
    pub speech: Option<i64>,
    /// The generated still it paid for.
    pub image: Option<i64>,
    /// The voice design it paid for (#572).
    pub design: Option<i64>,
    /// The music track it paid for (#1264).
    pub music: Option<i64>,
    /// The sound effect it paid for (#1265).
    pub sound: Option<i64>,
}

/// The user's paid balance, in micro-dollars: the sum of their entries. The
/// free allowance is not in it ([`allowance`]).
pub async fn balance(tx: &mut Tx) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COALESCE(sum(amount_micros), 0)::bigint FROM credit_entries")
        .fetch_one(&mut **tx)
        .await
}

/// What the user can spend now on anything but a Veo shot: the paid balance
/// and what is left of the free allowance. The assistant is told this, never
/// the split (#1530).
pub async fn spendable(tx: &mut Tx) -> Result<i64, sqlx::Error> {
    let paid = balance(tx).await?;
    if owed_at(tx, paid).await?.is_some() {
        return Ok(paid);
    }
    Ok(paid + allowance::left(tx).await?.left_micros)
}

/// Take the user's lock: their own `users` row, held until the transaction
/// ends, so two spends of one user's money are decided one after the other
/// and cannot both see the same last dollar. Another user's spending never
/// waits on it. The same lock orders what else one user may take only so
/// much of: assistant turns at once (`assistant::holds`) and storage
/// (`library::room`).
pub(crate) async fn lock(tx: &mut Tx) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1 FROM users WHERE id = (SELECT member_id()) FOR UPDATE")
        .execute(&mut **tx)
        .await
        .map(drop)
}

/// Take the user's lock, then fail unless `funds` cover `micros`; the part
/// of it the free allowance pays, spent first. When the allowance may pay
/// any of it, the lock on everyone's free spend is taken too
/// ([`allowance`]).
pub async fn cover(tx: &mut Tx, micros: i64, funds: Funds) -> Result<i64, CreditError> {
    lock(tx).await?;
    let paid = balance(tx).await?;
    if let Some(balance) = owed_at(tx, paid).await? {
        return Err(CreditError::Owed { balance });
    }
    if funds == Funds::PaidOnly {
        if paid >= micros {
            return Ok(0);
        }
        // A shot the user's free allowance would have covered says why it
        // does not, whatever is left of everyone's.
        let left = allowance::own_left(tx).await?;
        if paid.saturating_add(left) >= micros {
            return Err(CreditError::PaidOnly {
                paid,
                needed: micros,
            });
        }
        return Err(CreditError::Insufficient {
            balance: paid,
            needed: micros,
        });
    }
    let (own, everyones) = allowance::own_and_everyones(tx).await?;
    let free = own.min(everyones);
    if paid.saturating_add(free) < micros {
        let balance = paid.saturating_add(free);
        // A spend the user's own allowance would have covered says why.
        if paid.saturating_add(own) >= micros {
            return Err(CreditError::FreeUsedUp {
                balance,
                needed: micros,
            });
        }
        return Err(CreditError::Insufficient {
            balance,
            needed: micros,
        });
    }
    Ok(free.min(micros))
}

/// Reserve `micros` for something about to be paid for — refused, with
/// nothing written, when the balance does not cover it. The part the free
/// allowance covers is granted beside it, except for a Veo shot.
pub async fn reserve(
    tx: &mut Tx,
    micros: i64,
    memo: &str,
    link: Link,
) -> Result<Reservation, CreditError> {
    if micros <= 0 {
        return Err(CreditError::Invalid(
            "only a positive amount can be reserved".into(),
        ));
    }
    let free = cover(tx, micros, Funds::for_link(&link)).await?;
    let entry = sqlx::query_scalar(
        "INSERT INTO credit_entries
             (user_id, kind, amount_micros, project_id, veo_generation_id, speech_generation_id,
              image_generation_id, voice_design_id, music_generation_id, sound_generation_id,
              memo)
         VALUES (member_id(), 'reservation', $1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
    )
    .bind(-micros)
    .bind(link.project)
    .bind(link.veo)
    .bind(link.speech)
    .bind(link.image)
    .bind(link.design)
    .bind(link.music)
    .bind(link.sound)
    .bind(memo)
    .fetch_one(&mut **tx)
    .await?;
    allowance::grant(tx, entry, free).await?;
    Ok(Reservation { entry, micros })
}

/// Settle a reservation whose provider **failed**: give it all back — the
/// paid part to the balance, the free part to the allowance. Free.
pub async fn release(tx: &mut Tx, reservation: Reservation) -> Result<(), CreditError> {
    settle(tx, reservation, "release", reservation.micros).await?;
    allowance::give_back(tx, reservation.entry).await?;
    Ok(())
}

/// Settle a reservation whose provider **worked**: give it back, and charge
/// `micros` — the price, which for every provider today is what was reserved.
/// What the allowance granted for it stays spent.
pub async fn charge(tx: &mut Tx, reservation: Reservation, micros: i64) -> Result<(), CreditError> {
    settle(tx, reservation, "release", reservation.micros).await?;
    settle(tx, reservation, "charge", -micros).await
}

/// One settling entry, carrying the reservation's memo and links. Refused
/// when there is no such open reservation: the unique index on releases is
/// what makes a second settlement impossible.
async fn settle(
    tx: &mut Tx,
    reservation: Reservation,
    kind: &str,
    amount: i64,
) -> Result<(), CreditError> {
    let written = sqlx::query(
        "INSERT INTO credit_entries (user_id, kind, amount_micros, settles, project_id,
                                     veo_generation_id, speech_generation_id,
                                     image_generation_id, voice_design_id, music_generation_id,
                                     sound_generation_id, memo)
         SELECT member_id(), $1, $2, id, project_id, veo_generation_id, speech_generation_id,
                image_generation_id, voice_design_id, music_generation_id, sound_generation_id,
                memo
         FROM credit_entries WHERE id = $3 AND kind = 'reservation'",
    )
    .bind(kind)
    .bind(amount)
    .bind(reservation.entry)
    .execute(&mut **tx)
    .await;
    match written {
        Ok(done) if done.rows_affected() == 1 => Ok(()),
        Ok(_) => Err(CreditError::NotOpen(reservation.entry)),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => {
            Err(CreditError::NotOpen(reservation.entry))
        }
        Err(error) => Err(error.into()),
    }
}

/// Credit `micros` the operator received. A top-up records the dollars
/// credited and nothing else (#703): whatever currency the money arrived in,
/// the ledger is kept in dollars.
pub async fn top_up(tx: &mut Tx, micros: i64) -> Result<(), CreditError> {
    if micros <= 0 {
        return Err(CreditError::Invalid(
            "a top-up needs a positive amount".into(),
        ));
    }
    insert_credit(tx, "top_up", micros, "Top-up", json!({}), None).await
}

/// Credit `micros` a user paid for themselves through `payment` (#548). The
/// same top-up as [`top_up`], linked to its payment — and refused by a
/// unique index if that payment was credited already, so a payment can never
/// become two top-ups however its confirmations race.
pub async fn top_up_paid(tx: &mut Tx, micros: i64, payment: i64) -> Result<(), CreditError> {
    if micros <= 0 {
        return Err(CreditError::Invalid(
            "a top-up needs a positive amount".into(),
        ));
    }
    insert_credit(tx, "top_up", micros, "Top-up", json!({}), Some(payment)).await
}

/// Credit the bonus `micros` a larger purchase, `payment`, adds (#1537),
/// beside its top-up — refused by a unique index if that payment's bonus was
/// credited already.
pub async fn bonus(tx: &mut Tx, micros: i64, payment: i64) -> Result<(), CreditError> {
    if micros <= 0 {
        return Err(CreditError::Invalid(
            "a bonus needs a positive amount".into(),
        ));
    }
    insert_credit(tx, "bonus", micros, "Bonus", json!({}), Some(payment)).await
}

/// Take back `micros`, what was left of `payment` when it was withdrawn
/// (`payments::withdraw`) — refused by a unique index if it was withdrawn
/// already. Under the user's lock, so nothing is spent while it is decided.
pub async fn withdraw(
    tx: &mut Tx,
    micros: i64,
    payment: i64,
    detail: serde_json::Value,
) -> Result<(), CreditError> {
    if micros <= 0 {
        return Err(CreditError::Invalid(
            "a withdrawal takes back a positive amount".into(),
        ));
    }
    insert_credit(
        tx,
        "withdrawal",
        -micros,
        "Purchase withdrawn",
        detail,
        Some(payment),
    )
    .await
}

/// Take the user's lock and read their paid balance, for a decision that
/// must not race a spend.
pub async fn locked_balance(tx: &mut Tx) -> Result<i64, sqlx::Error> {
    lock(tx).await?;
    balance(tx).await
}

/// Give back `micros` with the reason in words — the refund entry kind the
/// refund policy (#547) acts through.
pub async fn refund(tx: &mut Tx, micros: i64, reason: &str) -> Result<(), CreditError> {
    if micros <= 0 || reason.trim().is_empty() {
        return Err(CreditError::Invalid(
            "a refund needs a positive amount and a reason".into(),
        ));
    }
    insert_credit(
        tx,
        "refund",
        micros,
        &format!("Refund: {reason}"),
        json!({}),
        None,
    )
    .await
}

/// One entry that is not a spend: `micros` signed as its kind is, linked to
/// `payment` when it is about one — what a dispute or a refund in Stripe
/// writes (`payments::reversed`), held to one per Stripe event by an index.
pub(crate) async fn insert_credit(
    tx: &mut Tx,
    kind: &str,
    micros: i64,
    memo: &str,
    detail: serde_json::Value,
    payment: Option<i64>,
) -> Result<(), CreditError> {
    sqlx::query(
        "INSERT INTO credit_entries (user_id, kind, amount_micros, memo, detail, payment_id)
         VALUES (member_id(), $1, $2, $3, $4, $5)",
    )
    .bind(kind)
    .bind(micros)
    .bind(memo)
    .bind(detail)
    .bind(payment)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
