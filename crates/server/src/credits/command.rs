//! `scorsese-server credit …`: the operator's side of the money.
//!
//! **Top-ups are manual in v1** (#527): somebody pays the maintainer, and the
//! maintainer records the dollars it credits — dollars only, whatever arrived
//! (#703). Pix (#548) will later write the same kind of entry. Every command
//! acts on one user's ledger **scoped as that user**, exactly as a request of
//! theirs would — only finding them by email is privileged.

use clap::Subcommand;
use sqlx::postgres::PgPool;

use super::{CreditError, adjust, dollars, history, ledger};
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
    /// Take test money back out (#1542), with the reason, as one new
    /// negative adjustment entry. Refuses an account that paid through
    /// Stripe unless `--include-paid` is given.
    Adjust {
        /// The account's email.
        email: String,
        /// Dollars to take back, written negative, e.g. -5 or -2.50.
        #[arg(
            long,
            allow_hyphen_values = true,
            conflicts_with = "to_zero",
            required_unless_present = "to_zero"
        )]
        dollars: Option<String>,
        /// Take the whole balance back, leaving it at zero.
        #[arg(long)]
        to_zero: bool,
        /// Why, in words the user will read in their history.
        #[arg(long)]
        reason: String,
        /// Adjust even an account with a payment that arrived through
        /// Stripe: without it, real purchases are never taken back.
        #[arg(long)]
        include_paid: bool,
    },
    /// Print a user's balance.
    Balance {
        /// The account's email.
        email: String,
    },
    /// Print what the free allowance paid, per user per Brasília day, with
    /// the month to date and when each user first paid (#1530).
    FreeUsage {
        /// Only this account's.
        email: Option<String>,
        /// Only days on or after this one, YYYY-MM-DD.
        #[arg(long)]
        since: Option<String>,
        /// Only days on or before this one, YYYY-MM-DD.
        #[arg(long)]
        until: Option<String>,
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
        CreditCommand::Adjust {
            email,
            dollars: amount,
            to_zero,
            reason,
            include_paid,
        } => {
            let take = match (amount, to_zero) {
                (Some(amount), false) => adjust::Take::Micros(parse_negative(&amount)?),
                (None, true) => adjust::Take::All,
                _ => {
                    return Err(CreditError::Invalid(
                        "give exactly one of --dollars -N and --to-zero".into(),
                    )
                    .into());
                }
            };
            let user = users::find(pool, &email).await?;
            let mut tx = db::scoped(pool, user)
                .await
                .map_err(ServerError::Database)?;
            let taken = adjust::adjust(&mut tx, take, &reason, include_paid)
                .await
                .map_err(|error| match error {
                    CreditError::Invalid(why) => CreditError::Invalid(format!("{email}: {why}")),
                    other => other,
                })?;
            let balance = ledger::balance(&mut tx)
                .await
                .map_err(ServerError::Database)?;
            tx.commit().await.map_err(ServerError::Database)?;
            if taken == 0 {
                format!(
                    "{email} is already at {}; nothing written",
                    dollars(balance)
                )
            } else {
                format!(
                    "took {} back from {email}; balance now {}",
                    dollars(taken),
                    dollars(balance)
                )
            }
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
        CreditCommand::FreeUsage {
            email,
            since,
            until,
        } => free_usage(pool, email.as_deref(), since.as_deref(), until.as_deref()).await?,
    })
}

/// One day of a user's free usage, as `free_usage.sql` reads it.
#[derive(sqlx::FromRow)]
struct Day {
    email: String,
    day: String,
    free: i64,
    month_to_date: i64,
    first_paid: Option<String>,
}

/// `credit free-usage`: each user's line, then one per day they used the
/// allowance.
async fn free_usage(
    pool: &PgPool,
    email: Option<&str>,
    since: Option<&str>,
    until: Option<&str>,
) -> Result<String, ServerError> {
    for day in [since, until].into_iter().flatten() {
        if !history::is_day(day) {
            return Err(CreditError::Invalid(format!(
                "{day:?} is not a day; write it as YYYY-MM-DD"
            ))
            .into());
        }
    }
    let mut tx = db::privileged(pool).await.map_err(ServerError::Database)?;
    let days: Vec<Day> = sqlx::query_as(include_str!("free_usage.sql"))
        .bind(email)
        .bind(since)
        .bind(until)
        .fetch_all(&mut *tx)
        .await
        .map_err(ServerError::Database)?;
    tx.commit().await.map_err(ServerError::Database)?;
    let mut lines = Vec::new();
    for (index, day) in days.iter().enumerate() {
        if index == 0 || days[index - 1].email != day.email {
            lines.push(match &day.first_paid {
                Some(first) => format!("{}, first paid {first}", day.email),
                None => format!("{}, never paid", day.email),
            });
        }
        lines.push(format!(
            "  {}  {} free, {} this month",
            day.day,
            dollars(day.free),
            dollars(day.month_to_date)
        ));
    }
    if lines.is_empty() {
        lines.push("nobody has used the free allowance in that range".to_owned());
    }
    Ok(lines.join("\n"))
}

/// An amount to take back, written negative as the operator types it —
/// `-5`, `-2.50` — as positive micro-dollars. A figure without the minus is
/// refused: an adjustment never adds.
fn parse_negative(text: &str) -> Result<i64, CreditError> {
    text.trim()
        .strip_prefix('-')
        .ok_or_else(|| {
            CreditError::Invalid(format!(
                "{text:?} does not take money back; write it negative, e.g. -5"
            ))
        })
        .and_then(|amount| {
            parse_decimal(amount, 6).map_err(|_| {
                CreditError::Invalid(format!(
                    "{text:?} is not an amount to take back; write it negative with at most \
                     6 decimal places, e.g. -2.50"
                ))
            })
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

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::CreditCommand;
    use crate::operator::{Cli, Command};

    fn parse(args: &[&str]) -> Result<CreditCommand, clap::Error> {
        let words = ["scorsese-server", "credit", "adjust", "ana@example.com"];
        let cli = Cli::try_parse_from(words.iter().chain(args))?;
        match cli.command {
            Some(Command::Credit(command)) => Ok(command),
            other => panic!("not a credit command: {other:?}"),
        }
    }

    /// `--dollars -5` reads as the amount, not as a flag, and exactly one of
    /// it and `--to-zero` is given (#1542).
    #[test]
    fn adjust_takes_a_negative_amount_or_to_zero() {
        let read = parse(&["--dollars", "-5", "--reason", "test"]).unwrap();
        assert!(
            matches!(&read, CreditCommand::Adjust { dollars: Some(d), to_zero: false, .. } if d == "-5"),
            "{read:?}"
        );
        let read = parse(&["--to-zero", "--reason", "test", "--include-paid"]).unwrap();
        assert!(
            matches!(
                read,
                CreditCommand::Adjust {
                    dollars: None,
                    to_zero: true,
                    include_paid: true,
                    ..
                }
            ),
            "{read:?}"
        );
        for args in [
            &["--reason", "test"][..],
            &["--to-zero", "--dollars", "-5", "--reason", "test"],
            &["--to-zero"],
        ] {
            assert!(parse(args).is_err(), "{args:?}");
        }
    }

    /// An adjustment never adds: the amount is written negative.
    #[test]
    fn only_a_negative_amount_is_taken_back() {
        assert_eq!(super::parse_negative("-2.50").unwrap(), 2_500_000);
        assert_eq!(super::parse_negative(" -5 ").unwrap(), 5_000_000);
        for not in ["5", "+5", "-0", "--5", "-"] {
            assert!(super::parse_negative(not).is_err(), "{not:?}");
        }
    }
}
