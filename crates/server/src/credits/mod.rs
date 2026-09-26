//! Credits: what each user has paid in and spent, and the record of every
//! paid generation (#537).
//!
//! Users pay up front; the maintainer pays Google, ElevenLabs and Anthropic up
//! front; every cent in between is accounted for **here, in the database** —
//! not in a provider's billing export a day late. The doctrine is
//! `docs/web.md` (*Money*); this is how it is kept.
//!
//! ## The ledger is append-only, and the database holds it so
//!
//! `credit_entries` is the truth, and a user's balance is the **sum** of their
//! entries. An entry is never edited; a correction is a new entry. That is
//! enforced by Postgres rather than by care: the member role has no `UPDATE`
//! or `DELETE` on the table at all, and a trigger refuses both to everybody
//! else too — the login role included — except the delete that cascades from
//! an account's own deletion (migration `0004_credits.sql`).
//!
//! Amounts are **integer micro-dollars** in a signed `BIGINT`: money in is
//! positive, money out negative. One assistant call can cost a fraction of a
//! cent, so cents are too coarse; a float is not the same sum twice.
//!
//! ## Spending: reserve, then settle
//!
//! A paid generation ([`generations`]) is priced by the provider tables in
//! `scorsese_providers::prices` — the same figure the quote (#538) showed —
//! plus [`MARKUP_PERCENT`]. Then:
//!
//! 1. **Reserve** the price: a negative entry, written only when the balance
//!    covers it. So two generations started at once cannot both spend the
//!    last dollar — each takes a lock on the user's own `users` row first.
//!    Refused means nothing was sent to the provider and nothing is recorded.
//! 2. **Settle** once the provider answers. A release gives the reservation
//!    back; if the generation **worked**, a charge takes the price. So a
//!    **failure is free** and a success is charged, liked or not (#527).
//!    A unique index allows one release per reservation, so nothing is
//!    settled twice — the ledger's own form of "never pay twice".
//!
//! A Veo job that goes `stuck` settles nothing: Google may still be
//! generating and billing, so the reservation stays held until the shot is
//! collected or given up on.
//!
//! **The assistant is charged after its call, not reserved for**
//! ([`ledger::charge_assistant`]): its cost is exact only once the response
//! has counted its tokens. Whoever runs a turn (#540) checks there is a
//! positive balance before starting one; the call itself may take the balance
//! a little below zero, which the next top-up absorbs.
//!
//! ## The monthly fee is a sweep, not a job
//!
//! [`fees`]: each month of an account's life, counted from its first top-up,
//! owes [`MONTHLY_FEE_MICROS`]. A loop in the server looks hourly for months
//! owed and not yet charged. Not a job in the queue: the queue is for long
//! work with a handler and crash recovery, and a fee is one idempotent insert
//! — a unique index on (user, month) makes charging twice impossible, and the
//! next sweep *is* the retry. Nor a cron outside the server: a home machine
//! that was off at the due moment must catch up on its own, and a sweep that
//! asks "what is owed now?" does that by construction.
//!
//! ## What the user sees
//!
//! [`history`] is the same ledger scoped to its owner and written for them:
//! one row per thing that moved their balance, the balance after each, in
//! ≈ reais at the operator's dated [`rates`] with dollars beside. It is served
//! at `GET /api/credits/history` and described as a read-only tool
//! ([`tool`]) for web MCP (#539) to register.

pub mod command;
pub mod fees;
pub mod generations;
pub mod history;
pub mod ledger;
pub mod rates;
pub mod tool;

pub use ledger::Reservation;

/// What scorsese adds to a provider's price: 10% (#527).
pub const MARKUP_PERCENT: u64 = 10;

/// The flat monthly fee, in micro-dollars: $10 (#527).
pub const MONTHLY_FEE_MICROS: i64 = 10_000_000;

/// Micro-dollars in a US cent. The provider tables speak cents; the ledger
/// speaks micro-dollars, and this is the one place they meet.
pub const MICROS_PER_CENT: u64 = 10_000;

/// What a user is charged for something that costs scorsese `cost_micros`:
/// the cost plus [`MARKUP_PERCENT`], **rounded up** (`docs/prices.md`) —
/// rounding down would give away a fraction of a micro-dollar at a time.
pub fn price(cost_micros: u64) -> i64 {
    let marked = u128::from(cost_micros) * u128::from(100 + MARKUP_PERCENT);
    i64::try_from(marked.div_ceil(100)).unwrap_or(i64::MAX)
}

/// A provider table's cents, as the ledger's micro-dollars.
pub fn from_cents(cents: u64) -> u64 {
    cents.saturating_mul(MICROS_PER_CENT)
}

/// Micro-dollars as dollars a person reads: `$1.06`, `-$0.03`, and as many
/// places past the cent as a fraction of one needs — `$0.0022`.
pub fn dollars(micros: i64) -> String {
    let sign = if micros < 0 { "-" } else { "" };
    let magnitude = micros.unsigned_abs();
    let whole = magnitude / 1_000_000;
    let fraction = format!("{:06}", magnitude % 1_000_000);
    let places = fraction.trim_end_matches('0').len().max(2);
    format!("{sign}${whole}.{}", &fraction[..places])
}

/// Why spending or recording credits did not happen.
#[derive(Debug, thiserror::Error)]
pub enum CreditError {
    /// The balance does not cover what this would reserve. Nothing was spent.
    #[error(
        "your balance is {}, and this needs {} — nothing was sent to the provider. \
         Add credit to go ahead.",
        dollars(*balance),
        dollars(*needed)
    )]
    Insufficient {
        /// What the balance was.
        balance: i64,
        /// What the generation would have reserved.
        needed: i64,
    },
    /// A reservation that does not exist, or was already settled.
    #[error("reservation {0} is not open: it does not exist or was already settled")]
    NotOpen(i64),
    /// The assistant ran on a model the rate table has no row for.
    #[error("there is no published rate for {0}, so the call cannot be charged")]
    Unpriced(String),
    /// An operator's figure that does not parse, or makes no sense.
    #[error("{0}")]
    Invalid(String),
    /// The database refused.
    #[error("the database refused: {0}")]
    Database(#[from] sqlx::Error),
}
