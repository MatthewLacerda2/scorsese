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
//! **A token keeps what it covers** ([`QuoteItem`]): every brief it would pay
//! for, with its price from the user's credits and the words it would send,
//! written when it is issued from the same quote it is bound to. That is what
//! the assistant's confirmation box lists, so the box shows exactly what its
//! yes spends, item for item, and nothing read back from the reply's words.
//!
//! **Taking a token is deleting its row.** Of two calls confirming one token
//! at once, one deletes it and the other finds nothing — spent at most once,
//! the same guarantee the local store gets from removing a file.

use std::convert::Infallible;
use std::sync::Mutex;

use scorsese_providers::quote::{self, Issued, Quote, Quotes, Refused, Spend};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;

use crate::db::Tx;

/// One brief a quote's token would pay for, as the confirmation box shows it:
/// the line the paid tool priced it with, what it takes from the balance, and
/// the words that would be sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteItem {
    /// What it is — an asset id, or `design` for a voice design.
    pub subject: String,
    /// What the paid tool said about it: the price, and how it got there.
    pub says: String,
    /// What it takes from the balance, in micro-dollars: the token's
    /// items add up to the token's whole price. Zero on a quote held before
    /// items carried a price.
    #[serde(default)]
    pub micros: i64,
    /// What `description` is.
    pub brief: BriefKind,
    /// The words that would be sent: a prompt, a line to speak, a voice's
    /// description.
    pub description: String,
}

/// What a quoted item's description is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BriefKind {
    /// A generated shot's or still's prompt.
    Prompt,
    /// A narration line, spoken as written.
    Line,
    /// A designed voice's description.
    Voice,
}

/// Issue a token for `quote` at `now`, kept in `tx`'s user's table with
/// `items`, every brief of `quote`'s it would pay for, and naming the tool
/// call `call` that asked for it. What it takes from the balance is the sum
/// of the items' prices.
pub(super) async fn issue(
    tx: &mut Tx,
    quote: &Quote,
    items: &[QuoteItem],
    now: i64,
    call: i64,
) -> Result<Issued, sqlx::Error> {
    let held = Held::default();
    let Ok(issued) = quote::issue(&held, quote, now);
    let micros: i64 = items.iter().map(|item| item.micros).sum();
    sqlx::query("DELETE FROM quotes WHERE expires_at < $1")
        .bind(now)
        .execute(&mut **tx)
        .await?;
    sqlx::query(
        "INSERT INTO quotes (token, user_id, spend, digest, cents, micros, issued_at,
                             expires_at, tool_call_id, items)
         VALUES ($1, member_id(), $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(&issued.token)
    .bind(issued.spend.as_str())
    .bind(&issued.digest)
    .bind(i64::try_from(issued.cents).unwrap_or(i64::MAX))
    .bind(micros)
    .bind(issued.issued_at)
    .bind(issued.expires_at)
    .bind(call)
    .bind(Json(items))
    .execute(&mut **tx)
    .await?;
    Ok(issued)
}

/// A quote a tool call issued and nobody has spent: its token, and when it
/// stops being good (seconds since the epoch).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// The token a confirming call hands back.
    pub token: String,
    /// What it takes from the balance, in micro-dollars.
    pub micros: i64,
    /// When it expires.
    pub expires_at: i64,
    /// What the half-price batch offered beside it takes, in micro-dollars
    /// (#947) — `None` when the call offered none. Its token is
    /// `Toolbox::batch_offer`'s to find, at the yes.
    pub batch_micros: Option<i64>,
    /// Every brief the token would pay for, in the order quoted; their
    /// prices add up to `micros`.
    pub items: Vec<QuoteItem>,
}

/// The quote tool call `call` issued, if it issued one still unspent.
///
/// A call issues at most two: its quote, and a batch offered beside a quote
/// for now ([`alternative`]). The offer is the one under `Spend::Batch` when
/// the other is not; a call that quoted a batch itself issued only that.
pub(super) async fn issued_by(tx: &mut Tx, call: i64) -> Result<Option<Pending>, sqlx::Error> {
    let rows: Vec<(String, i64, i64, Json<Vec<QuoteItem>>)> = sqlx::query_as(
        "SELECT token, micros, expires_at, items FROM quotes WHERE tool_call_id = $1
         ORDER BY spend = $2, token",
    )
    .bind(call)
    .bind(Spend::Batch.as_str())
    .fetch_all(&mut **tx)
    .await?;
    let mut rows = rows.into_iter();
    Ok(rows
        .next()
        .map(|(token, micros, expires_at, items)| Pending {
            token,
            micros,
            expires_at,
            batch_micros: rows.next().map(|(_, offered, ..)| offered),
            items: items.0,
        }))
}

/// The half-price batch offered beside the quote `token` (#947): its token,
/// while it is unspent.
pub(super) async fn alternative(tx: &mut Tx, token: &str) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT offer.token FROM quotes offer
         JOIN quotes quoted ON quoted.tool_call_id = offer.tool_call_id
         WHERE quoted.token = $1 AND offer.token <> $1 AND offer.spend = $2",
    )
    .bind(token)
    .bind(Spend::Batch.as_str())
    .fetch_optional(&mut **tx)
    .await
}

/// The arguments of the `tool` call that issued `token`, while it is unspent.
pub(super) async fn asked(
    tx: &mut Tx,
    token: &str,
    tool: &str,
) -> Result<Option<serde_json::Value>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT t.arguments FROM quotes q JOIN tool_calls t ON t.id = q.tool_call_id
         WHERE q.token = $1 AND t.tool = $2",
    )
    .bind(token)
    .bind(tool)
    .fetch_optional(&mut **tx)
    .await
}

/// Forget `token` unspent — a quote the user said no to — and the batch
/// offered beside it, if any: a no is to both.
pub(super) async fn withdraw(tx: &mut Tx, token: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM quotes WHERE token = $1
            OR tool_call_id = (SELECT tool_call_id FROM quotes WHERE token = $1)",
    )
    .bind(token)
    .execute(&mut **tx)
    .await?;
    Ok(())
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
    [Spend::Generation, Spend::VoiceDesign, Spend::Batch]
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
