//! The operator taking money back out of a balance (#1542): test credit
//! given by hand, reset before a deploy. One new negative `adjustment`
//! entry with its reason; the ledger stays append-only.

use super::{CreditError, dollars, ledger};
use crate::db::Tx;

/// How much an adjustment takes back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Take {
    /// This many micro-dollars: positive, at most the balance.
    Micros(i64),
    /// The whole balance, leaving it at zero.
    All,
}

/// What an adjustment found on the account, read in one statement.
#[derive(sqlx::FromRow)]
struct Holding {
    balance: i64,
    /// A payment that arrived through Stripe.
    paid: bool,
    /// A reservation no release or charge has settled yet.
    held: bool,
}

/// Take money back out with the reason in words (#1542): one new negative
/// `adjustment` entry, never an edit, never below zero. Returns what was
/// taken: nothing, with nothing written, when [`Take::All`] finds the balance
/// at zero or below.
///
/// Refused on an account with a Stripe payment unless `include_paid`, and
/// [`Take::All`] is refused while a generation holds a reservation, whose
/// failure would give it back after the reset. The balance and both checks
/// are one statement, so one snapshot: a payment settling meanwhile writes
/// its top-up and its `paid` state together, and is either in both or in
/// neither.
pub async fn adjust(
    tx: &mut Tx,
    take: Take,
    reason: &str,
    include_paid: bool,
) -> Result<i64, CreditError> {
    if reason.trim().is_empty() {
        return Err(CreditError::Invalid("an adjustment needs a reason".into()));
    }
    ledger::lock(tx).await?;
    let holding: Holding = sqlx::query_as(
        "SELECT (SELECT COALESCE(sum(amount_micros), 0)::bigint FROM credit_entries) AS balance,
                EXISTS (SELECT 1 FROM payments WHERE state = 'paid') AS paid,
                EXISTS (SELECT 1 FROM credit_entries r
                        WHERE r.kind = 'reservation'
                          AND NOT EXISTS (SELECT 1 FROM credit_entries s
                                          WHERE s.settles = r.id AND s.kind = 'release')) AS held",
    )
    .fetch_one(&mut **tx)
    .await?;
    if holding.paid && !include_paid {
        return Err(CreditError::Invalid(
            "this account has paid through Stripe; nothing was taken back. \
             Pass --include-paid to adjust it anyway"
                .into(),
        ));
    }
    let micros = match take {
        Take::All if holding.held => {
            return Err(CreditError::Invalid(
                "a generation is still running on this account, and its reservation would \
                 come back if it failed; nothing was taken back. Try again once it settles"
                    .into(),
            ));
        }
        Take::All => holding.balance,
        Take::Micros(micros) if micros <= 0 => {
            return Err(CreditError::Invalid(
                "an adjustment takes a positive amount back".into(),
            ));
        }
        Take::Micros(micros) if micros > holding.balance => {
            return Err(CreditError::Invalid(format!(
                "the balance is {}, so at most that can be taken back",
                dollars(holding.balance)
            )));
        }
        Take::Micros(micros) => micros,
    };
    if micros <= 0 {
        return Ok(0);
    }
    sqlx::query(
        "INSERT INTO credit_entries (user_id, kind, amount_micros, memo)
         VALUES (member_id(), 'adjustment', $1, $2)",
    )
    .bind(-micros)
    .bind(format!("Adjustment: {}", reason.trim()))
    .execute(&mut **tx)
    .await?;
    Ok(micros)
}
