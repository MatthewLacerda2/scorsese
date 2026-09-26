//! The $10 monthly fee: which months are owed, and charging them.
//!
//! **Each month of an account's life, counted from its first top-up, owes
//! [`MONTHLY_FEE_MICROS`](super::MONTHLY_FEE_MICROS)** — the first month the
//! moment the first money arrives. From the first top-up, not from the
//! account's creation: an account the operator made for somebody who never
//! paid has never owed anything.
//!
//! **A fee is charged only when the balance covers it**, because credits are
//! paid up front and the ledger never lends. A month whose fee the balance
//! does not cover is retried by every sweep for as long as that month lasts,
//! so a top-up mid-month pays it; a month that ends uncovered is not charged
//! afterwards. Both of those are the conservative reading of what #527 left
//! open, recorded as a question on #537 for the maintainer.
//!
//! Why a sweep rather than a queued job is argued in the module doc of
//! [`credits`](super). Finding who owes is cross-user by nature and runs
//! privileged; each charge runs scoped as its owner, under the same lock a
//! reservation takes.

use std::time::Duration;

use sqlx::postgres::PgPool;
use tokio::sync::watch;

use super::{CreditError, MONTHLY_FEE_MICROS, ledger};
use crate::db::{self, UserId};

/// How often the server looks for months owed. A fee is due at a moment, but
/// nothing hangs on the minute it is taken.
pub const SWEEP_EVERY: Duration = Duration::from_secs(60 * 60);

/// One month owed and not yet charged.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Owed {
    user: UserId,
    /// The month's first day, `YYYY-MM-DD`.
    period: String,
}

/// Charge every month owed now that its owner's balance covers. Returns who
/// was charged, and the first day of the month each fee pays for.
pub async fn sweep(pool: &PgPool) -> Result<Vec<(UserId, String)>, sqlx::Error> {
    let mut charged = Vec::new();
    for owed in owed(pool).await? {
        let mut tx = db::scoped(pool, owed.user).await?;
        match ledger::cover(&mut tx, MONTHLY_FEE_MICROS).await {
            Ok(_) => {}
            Err(CreditError::Database(error)) => return Err(error),
            Err(_) => continue, // Not covered yet: the next sweep asks again.
        }
        let period: Option<String> = sqlx::query_scalar(
            "INSERT INTO credit_entries (user_id, kind, amount_micros, fee_period, memo)
             VALUES (member_id(), 'monthly_fee', $1, $2::date, 'Monthly fee from ' || $2)
             ON CONFLICT (user_id, fee_period) WHERE kind = 'monthly_fee' DO NOTHING
             RETURNING fee_period::text",
        )
        .bind(-MONTHLY_FEE_MICROS)
        .bind(&owed.period)
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;
        if let Some(period) = period {
            charged.push((owed.user, period));
        }
    }
    Ok(charged)
}

/// Every user's current month, where it has not been charged. Privileged.
async fn owed(pool: &PgPool) -> Result<Vec<Owed>, sqlx::Error> {
    let mut tx = db::privileged(pool).await?;
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "WITH first AS (
             SELECT user_id, min(created_at AT TIME ZONE 'UTC') AS paid
             FROM credit_entries WHERE kind = 'top_up' GROUP BY user_id
         ), month AS (
             SELECT user_id, (paid + make_interval(months =>
                 (extract(year FROM age(now() AT TIME ZONE 'UTC', paid)) * 12
                  + extract(month FROM age(now() AT TIME ZONE 'UTC', paid)))::integer))::date
                 AS period
             FROM first
         )
         SELECT m.user_id, m.period::text FROM month m
         WHERE NOT EXISTS (SELECT 1 FROM credit_entries e
                           WHERE e.user_id = m.user_id AND e.kind = 'monthly_fee'
                             AND e.fee_period = m.period)
         ORDER BY m.user_id",
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows
        .into_iter()
        .map(|(user, period)| Owed {
            user: UserId::from_row(user),
            period,
        })
        .collect())
}

/// Sweep now and every [`SWEEP_EVERY`] until `stop` turns true. What the
/// server runs beside its job worker; a failed sweep is logged and retried.
pub async fn run(pool: PgPool, mut stop: watch::Receiver<bool>) {
    loop {
        match sweep(&pool).await {
            Ok(charged) => {
                for (user, period) in charged {
                    eprintln!(
                        "scorsese-server: charged user {} the monthly fee from {period}",
                        user.get()
                    );
                }
            }
            Err(error) => eprintln!("scorsese-server: the monthly-fee sweep failed: {error}"),
        }
        tokio::select! {
            _ = stop.wait_for(|stopping| *stopping) => return,
            () = tokio::time::sleep(SWEEP_EVERY) => {}
        }
    }
}
