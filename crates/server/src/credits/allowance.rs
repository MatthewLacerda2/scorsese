//! The free allowance (#1530): what every account may spend for nothing, so
//! much a day and at most so much a calendar month, both counted in Brasília
//! time (`America/Sao_Paulo`: the day turns at 00:00 there, the month on the
//! 1st). Paying users keep it.
//!
//! **It is never deposited.** It exists only at the moment of spending: when
//! a reservation or an assistant charge is written ([`ledger`](super::ledger)),
//! the part the allowance covers is written beside it as an `allowance` entry,
//! positive, settling that spend. So nothing unspent is ever granted, nothing
//! expires, and the balance is still the sum of the entries: it is the **paid
//! balance**, what the user's own money left them. What is left of the
//! allowance is computed from the grants of the day and the month ([`left`]).
//!
//! **Free first, then paid**, under the same lock on the user's row
//! [`cover`](super::ledger::cover) takes, so two spends at once cannot both
//! use the last free dollar. **A Veo shot is never paid from it**: the ledger
//! writes no grant for one, and a constraint refuses one that tries
//! (migration `0045_free_allowance.sql`), whoever asked — the built-in
//! assistant or a user's own client over web MCP.
//!
//! **A failure gives the free part back to the allowance**, never to the paid
//! balance: a release is followed by a negative `allowance` entry settling the
//! same reservation, one at most by a unique index, and a grant given back no
//! longer counts against its day. Otherwise failing generations on purpose
//! would turn free credit into paid credit.
//!
//! The figures are the server's configuration (`SCORSESE_FREE_DAILY`,
//! `SCORSESE_FREE_MONTHLY`), written to the one-row `free_allowance` table
//! each time the server starts ([`set`]), so the ledger reads them where it
//! reads everything else.

use scorsese_providers::credentials::Environment;
use scorsese_providers::prices::parse_dollars;
use serde::Serialize;
use sqlx::postgres::PgPool;

use crate::config::{ConfigError, FREE_DAILY, FREE_MONTHLY};
use crate::db::Tx;

/// What every account is given for nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Allowance {
    /// The most a Brasília day may take from it, in micro-dollars.
    pub daily_micros: i64,
    /// The most a calendar month may take from it, in micro-dollars.
    pub monthly_micros: i64,
}

impl Allowance {
    /// Nothing given away: what a server that was told nothing gives, and
    /// what a database holds until a server says otherwise.
    pub const NONE: Self = Self {
        daily_micros: 0,
        monthly_micros: 0,
    };

    /// The maintainer's figures (2026-10-24): $4 a day, $12 a month.
    pub const DEFAULT: Self = Self {
        daily_micros: 4_000_000,
        monthly_micros: 12_000_000,
    };
}

impl Allowance {
    /// What [`FREE_DAILY`] and [`FREE_MONTHLY`] say, each [`Self::DEFAULT`]'s
    /// figure when unset.
    pub fn from_environment(environment: &Environment) -> Result<Self, ConfigError> {
        Ok(Self {
            daily_micros: dollars(environment, FREE_DAILY, Self::DEFAULT.daily_micros)?,
            monthly_micros: dollars(environment, FREE_MONTHLY, Self::DEFAULT.monthly_micros)?,
        })
    }
}

/// The dollars, zero or more, `variable` holds, or `default` when unset.
fn dollars(
    environment: &Environment,
    variable: &'static str,
    default: i64,
) -> Result<i64, ConfigError> {
    let Some(value) = environment.get(variable) else {
        return Ok(default);
    };
    parse_dollars(value).ok_or_else(|| ConfigError::Free {
        variable,
        value: value.to_owned(),
    })
}

impl Default for Allowance {
    fn default() -> Self {
        Self::NONE
    }
}

/// What is left of a user's allowance at a moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct Free {
    /// What may still be spent for nothing, in micro-dollars: the less of
    /// what is left of the day and of the month, never below zero.
    pub left_micros: i64,
    /// When it next grows, in seconds since 1970: the next midnight in
    /// Brasília, or the 1st of the next month once the month's is spent;
    /// `0` when the server gives nothing away.
    pub renews_at: i64,
    /// What the day has taken of it so far, in micro-dollars.
    pub used_today_micros: i64,
    /// What the month has taken of it so far, in micro-dollars.
    pub used_this_month_micros: i64,
}

/// Give every account `allowance` from now on. The server's own login does
/// this as it starts; a member may not.
pub async fn set(pool: &PgPool, allowance: Allowance) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE free_allowance SET daily_micros = $1, monthly_micros = $2")
        .bind(allowance.daily_micros.max(0))
        .bind(allowance.monthly_micros.max(0))
        .execute(pool)
        .await
        .map(drop)
}

/// What is left of the user's allowance now.
pub async fn left(tx: &mut Tx) -> Result<Free, sqlx::Error> {
    read(tx, None).await
}

/// What is left of the user's allowance at `at`, in seconds since 1970 — a
/// day or a month boundary, in a test.
pub async fn left_at(tx: &mut Tx, at: i64) -> Result<Free, sqlx::Error> {
    read(tx, Some(at)).await
}

/// One reading of the allowance, at `at` or now.
async fn read(tx: &mut Tx, at: Option<i64>) -> Result<Free, sqlx::Error> {
    sqlx::query_as(include_str!("allowance.sql"))
        .bind(at)
        .fetch_one(&mut **tx)
        .await
}

/// Grant `micros` of the allowance to the spend `settles` — a reservation or
/// an assistant charge — carrying its links, so the history folds it in.
/// Nothing is written for nothing. Refused by a constraint for a Veo shot.
pub(super) async fn grant(tx: &mut Tx, settles: i64, micros: i64) -> Result<(), sqlx::Error> {
    if micros <= 0 {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO credit_entries (user_id, kind, amount_micros, settles, project_id,
                                     veo_generation_id, speech_generation_id,
                                     image_generation_id, voice_design_id, music_generation_id,
                                     chat_turn_id, memo)
         SELECT member_id(), 'allowance', $2, id, project_id, veo_generation_id,
                speech_generation_id, image_generation_id, voice_design_id,
                music_generation_id, chat_turn_id, memo
         FROM credit_entries WHERE id = $1",
    )
    .bind(settles)
    .bind(micros)
    .execute(&mut **tx)
    .await
    .map(drop)
}

/// Give back to the allowance what it granted to `reservation`, whose
/// provider failed: the same amount, negative, settling the same
/// reservation. Nothing when it granted nothing.
pub(super) async fn give_back(tx: &mut Tx, reservation: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO credit_entries (user_id, kind, amount_micros, settles, project_id,
                                     speech_generation_id, image_generation_id,
                                     voice_design_id, music_generation_id, memo)
         SELECT member_id(), 'allowance', -amount_micros, settles, project_id,
                speech_generation_id, image_generation_id, voice_design_id,
                music_generation_id, memo
         FROM credit_entries
         WHERE settles = $1 AND kind = 'allowance' AND amount_micros > 0",
    )
    .bind(reservation)
    .execute(&mut **tx)
    .await
    .map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(pairs: Vec<(&'static str, &'static str)>) -> Result<Allowance, ConfigError> {
        Allowance::from_environment(&Environment::of(pairs))
    }

    #[test]
    fn the_allowance_is_four_a_day_and_twelve_a_month_unless_told_otherwise() {
        assert_eq!(with(vec![]).unwrap(), Allowance::DEFAULT);
        let told = with(vec![(FREE_DAILY, "$2.50"), (FREE_MONTHLY, "0")]).unwrap();
        assert_eq!(told.daily_micros, 2_500_000);
        assert_eq!(told.monthly_micros, 0, "zero gives nothing away");
        for nonsense in ["-1", "four", "1e3"] {
            let error = with(vec![(FREE_DAILY, nonsense)]).unwrap_err();
            assert!(matches!(error, ConfigError::Free { .. }), "{nonsense}");
        }
        assert_eq!(
            with(vec![(FREE_MONTHLY, "lots")]).unwrap_err().to_string(),
            "SCORSESE_FREE_MONTHLY must be dollars like 4.00, or 0 for none, and \"lots\" is not"
        );
    }
}
