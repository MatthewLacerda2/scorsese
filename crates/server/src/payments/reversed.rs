//! A paid purchase Stripe takes back after the fact (#1637): disputed with
//! the card's bank, or refunded from Stripe's Dashboard. The rules are the
//! maintainer's (2026-10-31):
//!
//! - **A dispute takes back the whole purchase, its bonus included, even
//!   past zero**, less what a withdrawal or a refund took of it already. A
//!   paid balance a dispute leaves below zero spends nothing, free or paid
//!   (`ledger::owed`), until a purchase brings it back.
//! - **A dispute won gives back exactly what it took**; one lost or closed
//!   otherwise changes nothing more. An inquiry (a dispute Stripe opens as
//!   `warning_needs_response`) takes the same when it opens, and one that
//!   closes `warning_closed`, never a dispute, gives it back as a win does
//!   ([`GIVES_BACK`]): Stripe took nothing. What is given back is less what
//!   a refund made while it was open would have taken, so an inquiry closed
//!   by refunding the charge gives nothing back. A closing delivered before
//!   its opening is kept on the payment, and the opening then takes nothing.
//! - **A refund takes back what a withdrawal would** (`withdraw`): what is
//!   left of the purchase, never past zero, in the share of the reais not
//!   yet refunded that this refund returns. What a withdrawal said to refund
//!   counts as refunded already, so the refund that carries it out takes
//!   nothing more.
//!
//! Each is its own entry, linked to the payment, with a sentence the owner
//! reads in their history. Every entry names the Stripe event that wrote it,
//! and a unique index allows one per event; every decision is made holding
//! the owner's lock and the payment's row, so a repeated or reordered
//! delivery finds the work done.

use scorsese_providers::payments::{Charge, Dispute};
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{PaymentError, Payments, intents, reais};
use crate::credits::{dollars, ledger};
use crate::db::{self, Tx};

/// The closing statuses that give back what a dispute took: won, and an
/// inquiry that closed without becoming a dispute.
pub const GIVES_BACK: [&str; 2] = ["won", "warning_closed"];

/// Whether a dispute closed `status` gives back what it took.
fn gives_back(status: Option<&str>) -> bool {
    status.is_some_and(|status| GIVES_BACK.contains(&status))
}

/// What Stripe said happened to a paid charge.
#[derive(Debug, Clone, Copy)]
pub enum Reversal<'a> {
    /// It was disputed.
    Disputed(&'a Dispute),
    /// Its dispute closed.
    Closed(&'a Dispute),
    /// Some or all of it was refunded.
    Refunded(&'a Charge),
}

/// What acting on a reversal did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reversed {
    /// It moved the owner's balance by this much, in micro-dollars.
    Moved(i64),
    /// There was nothing to take or give back.
    Nothing,
    /// No purchase here was paid through its payment intent.
    Unknown,
}

/// A paid payment, as a reversal reads it under its row lock.
#[derive(sqlx::FromRow)]
struct Paid {
    id: i64,
    stripe_session: String,
    brl_centavos: i64,
    credit_micros: i64,
    bonus_micros: i64,
    dispute_closed: Option<String>,
    refunded_centavos: i64,
    /// The day it was paid, in Brasília: `30 Oct`.
    paid_on: String,
    /// What withdrawals, disputes and refunds took of it, net of a dispute
    /// won: positive.
    taken: i64,
    /// What its dispute took, if one did.
    disputed: Option<i64>,
    /// The reais counted as refunded when its dispute opened.
    refunded_before: Option<i64>,
    /// Whether its dispute was given back.
    won: bool,
    /// The reais its withdrawal said to refund.
    withdrawn_centavos: i64,
}

impl Paid {
    /// The reais counted as refunded: what Stripe refunded, or what a
    /// withdrawal said to refund when that is more.
    fn refunded(&self) -> i64 {
        self.refunded_centavos.max(self.withdrawn_centavos)
    }
}

/// Act on `reversal`, which Stripe event `event` carried.
pub async fn reverse(
    pool: &PgPool,
    payments: &Payments,
    event: &str,
    reversal: Reversal<'_>,
) -> Result<Reversed, PaymentError> {
    let intent = match reversal {
        Reversal::Disputed(dispute) | Reversal::Closed(dispute) => &dispute.payment_intent,
        Reversal::Refunded(charge) => &charge.payment_intent,
    };
    let Some(intent) = intent else {
        return Ok(Reversed::Unknown);
    };
    let Some(owner) = intents::owner(pool, payments, intent).await? else {
        return Ok(Reversed::Unknown);
    };
    let mut tx = db::scoped(pool, owner).await?;
    let balance = ledger::locked_balance(&mut tx).await?;
    let paid: Option<Paid> = sqlx::query_as(include_str!("reversed.sql"))
        .bind(intent)
        .fetch_optional(&mut *tx)
        .await?;
    let Some(paid) = paid else {
        return Ok(Reversed::Unknown);
    };
    let moved = match reversal {
        Reversal::Disputed(dispute) => disputed(&mut tx, &paid, event, dispute).await?,
        Reversal::Closed(dispute) => closed(&mut tx, &paid, event, dispute).await?,
        Reversal::Refunded(charge) => refunded(&mut tx, &paid, event, charge, balance).await?,
    };
    tx.commit().await?;
    Ok(moved.map_or(Reversed::Nothing, Reversed::Moved))
}

/// Take back the whole of `paid`, less what was taken of it already.
async fn disputed(
    tx: &mut Tx,
    paid: &Paid,
    event: &str,
    dispute: &Dispute,
) -> Result<Option<i64>, PaymentError> {
    let credited = paid.credit_micros.saturating_add(paid.bonus_micros);
    let take = credited.saturating_sub(paid.taken);
    if paid.disputed.is_some() || gives_back(paid.dispute_closed.as_deref()) || take <= 0 {
        return Ok(None);
    }
    let memo = format!(
        "R$ {} purchase of {} disputed with your card's bank: its {} of credit taken back",
        reais(paid.brl_centavos),
        paid.paid_on,
        dollars(take)
    );
    let detail = json!({
        "stripe_event": event,
        "session": paid.stripe_session,
        "dispute": dispute.id,
        "bonus_micros": share(paid.bonus_micros, take, credited),
        "refunded_before": paid.refunded(),
    });
    ledger::insert_credit(tx, "dispute", -take, &memo, detail, Some(paid.id)).await?;
    Ok(Some(-take))
}

/// Keep how `paid`'s dispute closed, and give back what it took if it
/// closed in our favour ([`GIVES_BACK`]).
async fn closed(
    tx: &mut Tx,
    paid: &Paid,
    event: &str,
    dispute: &Dispute,
) -> Result<Option<i64>, PaymentError> {
    sqlx::query("UPDATE payments SET dispute_closed = $2 WHERE id = $1")
        .bind(paid.id)
        .bind(&dispute.status)
        .execute(&mut **tx)
        .await?;
    let (Some(taken), true, false) = (paid.disputed, gives_back(Some(&dispute.status)), paid.won)
    else {
        return Ok(None);
    };
    // A refund made while it was open took nothing, since the dispute had
    // taken it all: what is given back is less what that refund would have
    // taken, the same share of the reais not refunded when it opened.
    let before = paid.refunded_before.unwrap_or(0);
    let open = paid.brl_centavos - before;
    let give = share(taken, open - (paid.refunded() - before), open);
    if give <= 0 {
        return Ok(None);
    }
    let memo = format!(
        "Dispute settled in your favour: {} given back",
        dollars(give)
    );
    let detail =
        json!({ "stripe_event": event, "session": paid.stripe_session, "dispute": dispute.id });
    ledger::insert_credit(tx, "dispute_won", give, &memo, detail, Some(paid.id)).await?;
    Ok(Some(give))
}

/// Take back from `paid` the share of what is left of it that `charge`'s
/// newly refunded reais are of those not refunded before.
async fn refunded(
    tx: &mut Tx,
    paid: &Paid,
    event: &str,
    charge: &Charge,
    balance: i64,
) -> Result<Option<i64>, PaymentError> {
    if charge.currency != "brl" || charge.amount != paid.brl_centavos {
        tracing::error!(
            "Stripe refunded charge {} of {} {} for payment {} ({}), which was BRL {} \
             centavos; nothing taken back — look at it in Stripe's Dashboard",
            charge.id,
            charge.amount,
            charge.currency,
            paid.id,
            paid.stripe_session,
            paid.brl_centavos
        );
        return Ok(None);
    }
    let now = charge.amount_refunded.clamp(0, paid.brl_centavos);
    if now <= paid.refunded_centavos {
        return Ok(None);
    }
    sqlx::query("UPDATE payments SET refunded_centavos = $2 WHERE id = $1")
        .bind(paid.id)
        .bind(now)
        .execute(&mut **tx)
        .await?;
    let before = paid.refunded();
    let new = now.max(paid.withdrawn_centavos) - before;
    let credited = paid.credit_micros.saturating_add(paid.bonus_micros);
    let left = balance.min(credited.saturating_sub(paid.taken)).max(0);
    let take = share(left, new, paid.brl_centavos - before);
    if take <= 0 {
        return Ok(None);
    }
    let memo = format!(
        "R$ {} of your R$ {} purchase of {} refunded: {} of its credit taken back",
        reais(now - paid.refunded_centavos),
        reais(paid.brl_centavos),
        paid.paid_on,
        dollars(take)
    );
    let detail = json!({
        "stripe_event": event,
        "session": paid.stripe_session,
        "charge": charge.id,
        "refunded_centavos": now,
        "bonus_micros": share(paid.bonus_micros, take, credited),
    });
    ledger::insert_credit(tx, "purchase_refunded", -take, &memo, detail, Some(paid.id)).await?;
    Ok(Some(-take))
}

/// `of` × `part` / `whole`, floored; 0 when `whole` is not positive.
fn share(of: i64, part: i64, whole: i64) -> i64 {
    if whole <= 0 {
        return 0;
    }
    let portion = i128::from(of) * i128::from(part) / i128::from(whole);
    i64::try_from(portion).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::share;

    #[test]
    fn a_share_is_floored_and_nothing_of_nothing() {
        assert_eq!(share(10_020_040, 2_500, 5_000), 5_010_020);
        assert_eq!(share(7, 1, 2), 3);
        assert_eq!(share(7, 1, 0), 0);
        assert_eq!(share(7, 1, -3), 0);
        assert_eq!(share(i64::MAX, 2, 2), i64::MAX);
    }
}
