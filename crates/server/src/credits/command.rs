//! `scorsese-server credit …`: the operator's side of the money.
//!
//! **Top-ups are manual in v1** (#527): somebody sends the maintainer reais,
//! and the maintainer records what arrived and the rate it was converted at.
//! Pix (#548) will later write the same kind of entry. Every command acts on
//! one user's ledger **scoped as that user**, exactly as a request of theirs
//! would — only finding them by email and setting the display rate are
//! privileged.

use clap::Subcommand;
use sqlx::postgres::PgPool;

use super::rates::{self, parse_decimal, reais};
use super::{dollars, ledger};
use crate::ServerError;
use crate::accounts::users;
use crate::db;

/// `scorsese-server credit …`
#[derive(Debug, Subcommand)]
pub enum CreditCommand {
    /// Credit money a user paid: the reais that arrived and the rate they were
    /// converted to dollars at. Prints the dollars credited and the balance.
    TopUp {
        /// The account's email.
        email: String,
        /// Reais received, e.g. 100 or 100.50.
        #[arg(long)]
        reais: String,
        /// Reais per dollar the money was converted at, e.g. 5.4321.
        #[arg(long)]
        rate: String,
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
    /// Set the reais-per-dollar rate balances are shown at, as of now.
    Rate {
        /// Reais per dollar, e.g. 5.4321.
        brl_per_usd: String,
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
            reais: paid,
            rate,
        } => {
            let centavos = parse_decimal(&paid, 2)?;
            let rate = parse_decimal(&rate, 4)?;
            let user = users::find(pool, &email).await?;
            let mut tx = db::scoped(pool, user)
                .await
                .map_err(ServerError::Database)?;
            let credited = ledger::top_up(&mut tx, centavos, rate).await?;
            let balance = ledger::balance(&mut tx)
                .await
                .map_err(ServerError::Database)?;
            tx.commit().await.map_err(ServerError::Database)?;
            format!(
                "credited {email} {} for {} at {}; balance now {}",
                dollars(credited),
                reais(centavos),
                rate_text(rate),
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
        CreditCommand::Rate { brl_per_usd } => {
            let rate = parse_decimal(&brl_per_usd, 4)?;
            rates::set(pool, rate).await?;
            format!(
                "balances are now shown at {} reais per dollar",
                rate_text(rate)
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

/// A rate in ten-thousandths, as the operator typed it: `5.4321`.
fn rate_text(e4: i64) -> String {
    format!("{}.{:04}", e4 / 10_000, e4 % 10_000)
}
