//! A user's own spending history: the ledger, scoped to them and written for
//! them.
//!
//! **One row per thing that moved their balance**, not one per entry. A paid
//! generation is three entries (a reservation, its release, the charge), and
//! a person asking "what did that shot cost?" wants one answer: *charged
//! $1.06*, or *free: the provider failed*, or *pending* while it runs. So the
//! entries that settle a reservation are folded into it, and the row carries
//! their net. **Each row shows the balance after it**, counted over the whole
//! ledger in order of the rows — never over the filtered view, which would
//! make the number depend on the question.
//!
//! Filterable by project, kind and a date range (UTC days, inclusive), with a
//! total over everything the filter matches — "this project has cost ≈ R$ …" —
//! whatever page of it is being read.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::postgres::PgPool;

use super::CreditError;
use super::ledger;
use super::rates::{self, DisplayRate};
use crate::db::{self, UserId};

/// What a row is about, as a filter names it.
pub const KINDS: &[&str] = &[
    "veo_shot",
    "spoken_line",
    "assistant",
    "top_up",
    "monthly_fee",
    "refund",
];

/// The most rows one page returns.
pub const MAX_ROWS: i64 = 500;

/// Which rows to return. Every field is optional; none means everything.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Filter {
    /// Only rows about this project.
    pub project: Option<i64>,
    /// Only rows of this kind — one of [`KINDS`].
    pub kind: Option<String>,
    /// Only rows on or after this UTC day, `YYYY-MM-DD`.
    pub since: Option<String>,
    /// Only rows on or before this UTC day, `YYYY-MM-DD`.
    pub until: Option<String>,
    /// Only rows older than this row id: the next page.
    pub before: Option<i64>,
    /// At most this many rows, newest first. Defaults to and caps at [`MAX_ROWS`].
    pub limit: Option<i64>,
}

/// One thing that moved the balance.
#[derive(Debug, Clone, PartialEq, Serialize, sqlx::FromRow)]
pub struct Row {
    /// Its first entry's id: what `before` pages by.
    pub id: i64,
    /// When, in seconds since the Unix epoch.
    pub at: i64,
    /// When, in UTC to the minute, for reading: `2026-09-25 14:03 UTC`.
    pub when: String,
    /// One of [`KINDS`].
    pub kind: String,
    /// `charged`, `free` (the provider failed), `pending`, or `credited`.
    pub status: String,
    /// The project it was spent on, if any. Kept when the project is deleted.
    pub project_id: Option<i64>,
    /// That project's name, while it exists.
    pub project_name: Option<String>,
    /// What it was, in words.
    pub memo: String,
    /// What it moved the balance by, in micro-dollars: negative is spent.
    pub amount_micros: i64,
    /// The balance once it had.
    pub balance_after_micros: i64,
    /// What set its price: model, resolution and seconds; model, voice and
    /// text; the assistant's token counts; a top-up's reais and rate.
    pub detail: Value,
    /// The amount in centavos at the display rate, when there is one.
    #[sqlx(skip)]
    pub amount_centavos: Option<i64>,
}

/// A page of history, and what the filter adds up to.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct History {
    /// The balance now, in micro-dollars.
    pub balance_micros: i64,
    /// The balance in centavos at the display rate, when there is one.
    pub balance_centavos: Option<i64>,
    /// The rate amounts are shown at, and when it was set.
    pub rate: Option<DisplayRate>,
    /// How many rows the filter matches, across every page.
    pub matched: i64,
    /// What they add up to, in micro-dollars: negative is spent.
    pub total_micros: i64,
    /// The total in centavos at the display rate.
    pub total_centavos: Option<i64>,
    /// This page, newest first.
    pub rows: Vec<Row>,
}

/// A row as the query returns it, with the filter's totals beside it.
#[derive(sqlx::FromRow)]
struct Counted {
    #[sqlx(flatten)]
    row: Row,
    matched: i64,
    total_micros: i64,
}

/// `user`'s history, as `filter` asks.
pub async fn read(pool: &PgPool, user: UserId, filter: &Filter) -> Result<History, CreditError> {
    check(filter)?;
    let mut tx = db::scoped(pool, user).await?;
    let balance = ledger::balance(&mut tx).await?;
    let rate = rates::current(&mut tx).await?;
    let counted: Vec<Counted> = sqlx::query_as(include_str!("history.sql"))
        .bind(filter.project)
        .bind(filter.kind.as_deref())
        .bind(filter.since.as_deref())
        .bind(filter.until.as_deref())
        .bind(filter.before)
        .bind(filter.limit.unwrap_or(MAX_ROWS).clamp(1, MAX_ROWS))
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    let (matched, total) = counted
        .first()
        .map_or((0, 0), |first| (first.matched, first.total_micros));
    let in_reais = |micros: i64| rate.map(|rate| rate.centavos(micros));
    Ok(History {
        balance_micros: balance,
        balance_centavos: in_reais(balance),
        rate,
        matched,
        total_micros: total,
        total_centavos: in_reais(total),
        rows: counted
            .into_iter()
            .map(|counted| Row {
                amount_centavos: in_reais(counted.row.amount_micros),
                ..counted.row
            })
            .collect(),
    })
}

/// Refuse a filter the query would choke on, in words for whoever sent it.
fn check(filter: &Filter) -> Result<(), CreditError> {
    if let Some(kind) = &filter.kind
        && !KINDS.contains(&kind.as_str())
    {
        return Err(CreditError::Invalid(format!(
            "{kind:?} is not a kind of history row; it is one of {}",
            KINDS.join(", ")
        )));
    }
    for day in [&filter.since, &filter.until].into_iter().flatten() {
        if !is_day(day) {
            return Err(CreditError::Invalid(format!(
                "{day:?} is not a day; write it as YYYY-MM-DD"
            )));
        }
    }
    Ok(())
}

/// Whether `text` is a real calendar day written `YYYY-MM-DD`.
fn is_day(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    let [year, month, day] = parts.as_slice() else {
        return false;
    };
    let number = |part: &str, width: usize| {
        (part.len() == width && part.bytes().all(|b| b.is_ascii_digit()))
            .then(|| part.parse::<u32>().ok())
            .flatten()
    };
    let (Some(year), Some(month), Some(day)) = (number(year, 4), number(month, 2), number(day, 2))
    else {
        return false;
    };
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        1..=12 => 31,
        _ => return false,
    };
    (1..=days).contains(&day)
}

#[cfg(test)]
mod tests {
    use super::is_day;

    #[test]
    fn a_day_is_a_real_calendar_day() {
        for day in ["2026-09-25", "2024-02-29", "2000-02-29", "2026-12-31"] {
            assert!(is_day(day), "{day}");
        }
        for not in [
            "2026-02-29",
            "1900-02-29",
            "2026-13-01",
            "2026-04-31",
            "2026-9-25",
            "2026-09-00",
            "today",
            "2026-09-25; DROP",
        ] {
            assert!(!is_day(not), "{not}");
        }
    }
}
