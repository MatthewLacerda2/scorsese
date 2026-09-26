//! Writing to the ledger, and reading a balance off it.
//!
//! Every function takes the caller's transaction, scoped to the user it acts
//! for (`db::scoped`), so an entry and whatever it belongs to — a generation
//! row, a job — are committed together or not at all. Owners are written as
//! `member_id()`, never passed in.

use scorsese_providers::prices::claude::{self, Usage};
use serde_json::json;

use super::{CreditError, price};
use crate::db::Tx;

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
}

/// The user's balance, in micro-dollars: the sum of their entries.
pub async fn balance(tx: &mut Tx) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COALESCE(sum(amount_micros), 0)::bigint FROM credit_entries")
        .fetch_one(&mut **tx)
        .await
}

/// Take the user's lock, then fail unless the balance covers `micros`.
///
/// The lock is their own `users` row, held until the transaction ends, so two
/// spends of one user's money are decided one after the other and cannot both
/// see the same last dollar. Another user's spending never waits on it.
pub async fn cover(tx: &mut Tx, micros: i64) -> Result<i64, CreditError> {
    sqlx::query("SELECT 1 FROM users WHERE id = (SELECT member_id()) FOR UPDATE")
        .execute(&mut **tx)
        .await?;
    let balance = balance(tx).await?;
    if balance < micros {
        return Err(CreditError::Insufficient {
            balance,
            needed: micros,
        });
    }
    Ok(balance)
}

/// Reserve `micros` for something about to be paid for — refused, with
/// nothing written, when the balance does not cover it.
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
    cover(tx, micros).await?;
    let entry = sqlx::query_scalar(
        "INSERT INTO credit_entries
             (user_id, kind, amount_micros, project_id, veo_generation_id, speech_generation_id, memo)
         VALUES (member_id(), 'reservation', $1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(-micros)
    .bind(link.project)
    .bind(link.veo)
    .bind(link.speech)
    .bind(memo)
    .fetch_one(&mut **tx)
    .await?;
    Ok(Reservation { entry, micros })
}

/// Settle a reservation whose provider **failed**: give it all back. Free.
pub async fn release(tx: &mut Tx, reservation: Reservation) -> Result<(), CreditError> {
    settle(tx, reservation, "release", reservation.micros).await
}

/// Settle a reservation whose provider **worked**: give it back, and charge
/// `micros` — the price, which for every provider today is what was reserved.
pub async fn charge(tx: &mut Tx, reservation: Reservation, micros: i64) -> Result<(), CreditError> {
    release(tx, reservation).await?;
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
                                     veo_generation_id, speech_generation_id, memo)
         SELECT member_id(), $1, $2, id, project_id, veo_generation_id, speech_generation_id, memo
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

/// An assistant call, charged from the tokens its response reported.
#[derive(Debug, Clone)]
pub struct AssistantCall<'a> {
    /// The model it ran on, as the API names it.
    pub model: &'a str,
    /// The response's `usage`.
    pub usage: Usage,
    /// The project the turn was about.
    pub project: Option<i64>,
    /// What the user asked, in their words — what the history shows.
    pub prompt: &'a str,
}

/// Charge one assistant call: its exact cost plus the markup. Not reserved
/// for, and not refused — the tokens are already spent (see the module doc
/// of [`credits`](super)). Returns the micro-dollars charged.
pub async fn charge_assistant(tx: &mut Tx, call: &AssistantCall<'_>) -> Result<i64, CreditError> {
    let rate = claude::rate(call.model).ok_or_else(|| CreditError::Unpriced(call.model.into()))?;
    let cost = call.usage.micros(rate);
    let charged = price(cost);
    let detail = json!({
        "model": call.model,
        "usage": call.usage,
        "cost_micros": cost,
        "prompt": call.prompt,
    });
    sqlx::query(
        "INSERT INTO credit_entries (user_id, kind, amount_micros, project_id, memo, detail)
         VALUES (member_id(), 'charge', $1, $2, $3, $4)",
    )
    .bind(-charged)
    .bind(call.project)
    .bind(format!("Assistant: {}", call.model))
    .bind(detail)
    .execute(&mut **tx)
    .await?;
    Ok(charged)
}

/// Credit money the operator received: `reais_centavos` at `brl_per_usd_e4`
/// reais per dollar (in ten-thousandths). Returns the micro-dollars credited —
/// **rounded down**, the one place that direction is right: never credit a
/// fraction of a micro-dollar nobody paid.
pub async fn top_up(
    tx: &mut Tx,
    reais_centavos: i64,
    brl_per_usd_e4: i64,
) -> Result<i64, CreditError> {
    if reais_centavos <= 0 || brl_per_usd_e4 <= 0 {
        return Err(CreditError::Invalid(
            "a top-up needs a positive amount and a positive rate".into(),
        ));
    }
    // centavos / 100 reais ÷ (rate / 10⁴) dollars × 10⁶ micro-dollars.
    let micros = i128::from(reais_centavos) * 100_000_000 / i128::from(brl_per_usd_e4);
    let micros = i64::try_from(micros)
        .map_err(|_| CreditError::Invalid("that top-up is too large".into()))?;
    let detail = json!({ "reais_centavos": reais_centavos, "brl_per_usd_e4": brl_per_usd_e4 });
    insert_credit(tx, "top_up", micros, "Top-up", detail).await?;
    Ok(micros)
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
    )
    .await
}

/// One entry that adds money.
async fn insert_credit(
    tx: &mut Tx,
    kind: &str,
    micros: i64,
    memo: &str,
    detail: serde_json::Value,
) -> Result<(), CreditError> {
    sqlx::query(
        "INSERT INTO credit_entries (user_id, kind, amount_micros, memo, detail)
         VALUES (member_id(), $1, $2, $3, $4)",
    )
    .bind(kind)
    .bind(micros)
    .bind(memo)
    .bind(detail)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
