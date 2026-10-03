//! `scorsese-server credit …`: the operator's side of the money.
//!
//! **Top-ups are manual in v1** (#527): somebody pays the maintainer, and the
//! maintainer records the dollars it credits — dollars only, whatever arrived
//! (#703). Pix (#548) will later write the same kind of entry. Every command
//! acts on one user's ledger **scoped as that user**, exactly as a request of
//! theirs would — only finding them by email is privileged.

use clap::Subcommand;
use sqlx::postgres::PgPool;

use super::{CreditError, dollars, ledger};
use crate::ServerError;
use crate::accounts::users;
use crate::db;

/// `scorsese-server credit …`
#[derive(Debug, Subcommand)]
pub enum CreditCommand {
    /// Credit money a user paid, in dollars. Prints the balance.
    TopUp {
        /// The account's email.
        email: String,
        /// Dollars to credit, e.g. 20 or 18.50.
        #[arg(long)]
        dollars: String,
    },
    /// Give a user back dollars, with the reason, as a refund entry.
    Refund {
        /// The account's email.
        email: String,
        /// Dollars to give back, e.g. 1.06.
        #[arg(long)]
        dollars: String,
        /// Why, in words the user will read in their history.
        #[arg(long)]
        reason: String,
    },
    /// Print a user's balance.
    Balance {
        /// The account's email.
        email: String,
    },
}

/// Carry out a `credit` command. Returns what to print.
pub async fn run(pool: &PgPool, command: CreditCommand) -> Result<String, ServerError> {
    Ok(match command {
        CreditCommand::TopUp {
            email,
            dollars: amount,
        } => {
            let micros = parse_decimal(&amount, 6)?;
            let user = users::find(pool, &email).await?;
            let mut tx = db::scoped(pool, user)
                .await
                .map_err(ServerError::Database)?;
            ledger::top_up(&mut tx, micros).await?;
            let balance = ledger::balance(&mut tx)
                .await
                .map_err(ServerError::Database)?;
            tx.commit().await.map_err(ServerError::Database)?;
            format!(
                "credited {email} {}; balance now {}",
                dollars(micros),
                dollars(balance)
            )
        }
        CreditCommand::Refund {
            email,
            dollars: amount,
            reason,
        } => {
            let micros = parse_decimal(&amount, 6)?;
            let user = users::find(pool, &email).await?;
            let mut tx = db::scoped(pool, user)
                .await
                .map_err(ServerError::Database)?;
            ledger::refund(&mut tx, micros, &reason).await?;
            let balance = ledger::balance(&mut tx)
                .await
                .map_err(ServerError::Database)?;
            tx.commit().await.map_err(ServerError::Database)?;
            format!(
                "refunded {email} {}; balance now {}",
                dollars(micros),
                dollars(balance)
            )
        }
        CreditCommand::Balance { email } => {
            let user = users::find(pool, &email).await?;
            let mut tx = db::scoped(pool, user)
                .await
                .map_err(ServerError::Database)?;
            let balance = ledger::balance(&mut tx)
                .await
                .map_err(ServerError::Database)?;
            tx.commit().await.map_err(ServerError::Database)?;
            format!("{email}: {}", dollars(balance))
        }
    })
}

/// A decimal the operator typed — `100`, `100.5`, `1.06` — as an integer of
/// `places` decimal places. Either `.` or `,` separates the fraction, since an
/// operator may write `100,50`. Refuses more places than `places`, a
/// sign, and anything that is not digits: a figure about money that does not
/// parse exactly is not guessed at.
fn parse_decimal(text: &str, places: u32) -> Result<i64, CreditError> {
    let invalid = || {
        CreditError::Invalid(format!(
            "{text:?} is not a positive amount with at most {places} decimal places"
        ))
    };
    let (whole, fraction) = text
        .trim()
        .split_once(['.', ','])
        .unwrap_or((text.trim(), ""));
    let digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    if whole.is_empty() || !digits(whole) || !digits(fraction) || fraction.len() > places as usize {
        return Err(invalid());
    }
    let scale = 10_i64.pow(places);
    let padded = format!("{fraction:0<width$}", width = places as usize);
    let whole: i64 = whole.parse().map_err(|_| invalid())?;
    let fraction: i64 = if padded.is_empty() {
        0
    } else {
        padded.parse().map_err(|_| invalid())?
    };
    whole
        .checked_mul(scale)
        .and_then(|scaled| scaled.checked_add(fraction))
        .filter(|value| *value > 0)
        .ok_or_else(invalid)
}
