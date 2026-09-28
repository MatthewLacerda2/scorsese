//! Where the hosted server keeps a paid tool's quotes: the `quotes` table.
//!
//! The rules — bound to exactly what was quoted, single use, fifteen minutes —
//! are `scorsese_providers::quote`'s, and they stay there: [`issue`] and
//! [`redeem`] here call its `issue` and `redeem`, through a one-record store
//! standing in for the table, so this module decides nothing a local project's
//! `cache/quotes/` would decide differently. The table is only where the
//! record lives between the two calls: per user, row-level security like
//! every other, so a token issued to one user is unknown to every other, and
//! a row rather than memory so a restart between the quote and the yes costs
//! the yes nothing.
//!
//! **Taking a token is deleting its row.** Of two calls confirming one token
//! at once, one deletes it and the other finds nothing — spent at most once,
//! the same guarantee the local store gets from removing a file.

use std::convert::Infallible;
use std::sync::Mutex;

use scorsese_providers::quote::{self, Issued, Quote, Quotes, Refused, Spend};

use crate::db::Tx;

/// Issue a token for `quote` at `now`, kept in `tx`'s user's table.
pub(super) async fn issue(tx: &mut Tx, quote: &Quote, now: i64) -> Result<Issued, sqlx::Error> {
    let held = Held::default();
    let Ok(issued) = quote::issue(&held, quote, now);
    sqlx::query("DELETE FROM quotes WHERE expires_at < $1")
        .bind(now)
        .execute(&mut **tx)
        .await?;
    sqlx::query(
        "INSERT INTO quotes (token, user_id, spend, digest, cents, issued_at, expires_at)
         VALUES ($1, member_id(), $2, $3, $4, $5, $6)",
    )
    .bind(&issued.token)
    .bind(issued.spend.as_str())
    .bind(&issued.digest)
    .bind(i64::try_from(issued.cents).unwrap_or(i64::MAX))
    .bind(issued.issued_at)
    .bind(issued.expires_at)
    .execute(&mut **tx)
    .await?;
    Ok(issued)
}

/// Spend `token` against `current`, the quote as it stands now — or say why
/// not. The row goes either way, as every attempt consumes a token.
pub(super) async fn redeem(
    tx: &mut Tx,
    token: &str,
    current: &Quote,
    now: i64,
) -> Result<Result<Issued, String>, sqlx::Error> {
    let row: Option<(String, String, String, i64, i64, i64)> = sqlx::query_as(
        "DELETE FROM quotes WHERE token = $1
         RETURNING token, spend, digest, cents, issued_at, expires_at",
    )
    .bind(token)
    .fetch_optional(&mut **tx)
    .await?;
    let taken = row.and_then(|(token, spend, digest, cents, issued_at, expires_at)| {
        Some(Issued {
            token,
            spend: spend_named(&spend)?,
            digest,
            cents: u64::try_from(cents).unwrap_or_default(),
            issued_at,
            expires_at,
        })
    });
    let held = Held(Mutex::new(taken));
    Ok(quote::redeem(&held, token, current, now).map_err(|refused| said(&refused)))
}

/// A refusal in the words the local store's would read in.
fn said(refused: &Refused<Infallible>) -> String {
    refused.to_string()
}

/// A kind of spending, by the name the table stores.
fn spend_named(name: &str) -> Option<Spend> {
    [Spend::Generation, Spend::VoiceDesign]
        .into_iter()
        .find(|spend| spend.as_str() == name)
}

/// One record, standing in for the table while the providers' rules run.
#[derive(Default)]
struct Held(Mutex<Option<Issued>>);

impl Quotes for Held {
    type Error = Infallible;

    fn put(&self, issued: &Issued) -> Result<(), Infallible> {
        *self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(issued.clone());
        Ok(())
    }

    fn take(&self, token: &str) -> Result<Option<Issued>, Infallible> {
        let mut held = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok(held.take().filter(|issued| issued.token == token))
    }
}
