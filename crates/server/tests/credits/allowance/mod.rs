//! The free allowance (#1530): free credit each Brasília day, capped each
//! calendar month, spent before the paid balance and never on a Veo shot —
//! and never turned into paid credit by a generation that fails.

mod calendar;
mod failing;
mod spending;
mod veo;

use scorsese_server::credits::allowance::{self, Allowance};
use scorsese_server::credits::generations::{Line, Request};
use scorsese_server::db::{self, UserId};
use serde_json::Value;
use sqlx::postgres::PgPool;

/// The maintainer's figures: $4 a day, $12 a month.
const FOUR_A_DAY: Allowance = Allowance {
    daily_micros: 4_000_000,
    monthly_micros: 12_000_000,
};

/// A spoken line quoted at `cents`, which costs the user `cents` × 1.8.
pub(super) fn line(cents: u64, settings: &Value) -> Request<'_> {
    Request::Line(Line {
        project: None,
        tool_call: None,
        job: None,
        model: "fast",
        voice: "matilda",
        text: "Olá, mundo.",
        settings,
        estimated_cents: cents,
    })
}

/// What is left of `user`'s free allowance now.
async fn free(pool: &PgPool, user: UserId) -> i64 {
    let mut tx = db::scoped(pool, user).await.unwrap();
    allowance::left(&mut tx).await.unwrap().left_micros
}

/// What is left of `user`'s free allowance at `at`, in seconds since 1970.
async fn free_at(pool: &PgPool, user: UserId, at: i64) -> allowance::Free {
    let mut tx = db::scoped(pool, user).await.unwrap();
    allowance::left_at(&mut tx, at).await.unwrap()
}

/// A $1 line that the allowance paid for in full at `at`, written as the
/// ledger writes one: its reservation and the allowance beside it.
pub(super) async fn spent_free_at(pool: &PgPool, user: UserId, at: &str, micros: i64) {
    let mut tx = db::scoped(pool, user).await.unwrap();
    let reservation: i64 = sqlx::query_scalar(
        "INSERT INTO credit_entries (user_id, kind, amount_micros, memo, created_at)
         VALUES (member_id(), 'reservation', $1, 'a line', $2::timestamptz) RETURNING id",
    )
    .bind(-micros)
    .bind(at)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO credit_entries (user_id, kind, amount_micros, settles, memo, created_at)
         VALUES (member_id(), 'allowance', $1, $2, 'a line', $3::timestamptz)",
    )
    .bind(micros)
    .bind(reservation)
    .bind(at)
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

/// Seconds since 1970 of an RFC 3339 instant, as Postgres reads it.
async fn epoch(pool: &PgPool, at: &str) -> i64 {
    sqlx::query_scalar("SELECT extract(epoch FROM $1::timestamptz)::bigint")
        .bind(at)
        .fetch_one(pool)
        .await
        .unwrap()
}
