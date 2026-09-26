//! The display rate: reais per dollar, set by the operator, dated.
//!
//! The ledger is in dollars because the providers bill in dollars. Users think
//! in reais, so a balance is **shown** as "≈ R$ …" at the newest rate the
//! operator set — approximately, and with the date the rate was set, because
//! the dollar moves and the page must not imply a fixed amount (#527). It is
//! display only: nothing is charged or credited at it. A top-up records the
//! rate its own money was actually converted at, beside it.
//!
//! One rate for everybody, so `display_rates` is the one table that is not per
//! user (`tests/isolation.rs` names it): members read it and write nothing;
//! the operator sets it, privileged.

use serde::Serialize;
use sqlx::postgres::PgPool;

use super::CreditError;
use crate::db::{self, Tx};

/// The rate balances are shown at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct DisplayRate {
    /// Reais per dollar, in ten-thousandths: 5.4321 is 54321.
    pub brl_per_usd_e4: i64,
    /// When the operator set it, in seconds since the Unix epoch.
    pub set_at: i64,
}

impl DisplayRate {
    /// `micros` in whole centavos at this rate, to the nearest.
    pub fn centavos(self, micros: i64) -> i64 {
        // micro-dollars ÷ 10⁶ × (rate ÷ 10⁴) reais × 100 centavos.
        let exact = i128::from(micros) * i128::from(self.brl_per_usd_e4);
        let half = if exact < 0 { -50_000_000 } else { 50_000_000 };
        i64::try_from((exact + half) / 100_000_000).unwrap_or(i64::MAX)
    }
}

/// The newest rate, or `None` before the operator has set one.
pub async fn current(tx: &mut Tx) -> Result<Option<DisplayRate>, sqlx::Error> {
    sqlx::query_as(
        "SELECT brl_per_usd_e4, extract(epoch FROM set_at)::bigint AS set_at
         FROM display_rates ORDER BY id DESC LIMIT 1",
    )
    .fetch_optional(&mut **tx)
    .await
}

/// Set the rate, as of now. Privileged: the operator's, and nobody's own row.
pub async fn set(pool: &PgPool, brl_per_usd_e4: i64) -> Result<DisplayRate, CreditError> {
    if brl_per_usd_e4 <= 0 {
        return Err(CreditError::Invalid("a rate must be positive".into()));
    }
    let mut tx = db::privileged(pool).await?;
    let rate = sqlx::query_as(
        "INSERT INTO display_rates (brl_per_usd_e4) VALUES ($1)
         RETURNING brl_per_usd_e4, extract(epoch FROM set_at)::bigint AS set_at",
    )
    .bind(brl_per_usd_e4)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rate)
}

/// Centavos as a Brazilian reader writes them: `R$ 1.234,56`.
pub fn reais(centavos: i64) -> String {
    let sign = if centavos < 0 { "-" } else { "" };
    let magnitude = centavos.unsigned_abs();
    let digits = (magnitude / 100).to_string();
    let mut whole = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            whole.push('.');
        }
        whole.push(digit);
    }
    format!("{sign}R$ {whole},{:02}", magnitude % 100)
}

/// A decimal the operator typed — `100`, `100.5`, `5.4321` — as an integer of
/// `places` decimal places. Either `.` or `,` separates the fraction, since a
/// Brazilian operator writes `5,43`. Refuses more places than `places`, a
/// sign, and anything that is not digits: a figure about money that does not
/// parse exactly is not guessed at.
pub fn parse_decimal(text: &str, places: u32) -> Result<i64, CreditError> {
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
