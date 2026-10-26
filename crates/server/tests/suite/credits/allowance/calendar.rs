//! The day and the month turn in Brasília time.

use scorsese_server::credits::allowance;
use sqlx::postgres::PgPool;

use super::super::account;
use super::{FOUR_A_DAY, epoch, free_at, spent_free_at};

/// The day turns at midnight in Brasília, 03:00 UTC.
#[sqlx::test]
async fn midnight_in_brasilia_renews_the_day(pool: PgPool) {
    allowance::set(&pool, FOUR_A_DAY).await.unwrap();
    let ana = account(&pool, "ana@example.com").await;
    // 21:00 on the 24th in Brasília, though already the 25th in UTC.
    spent_free_at(&pool, ana, "2026-10-25T00:00:00Z", 3_000_000).await;

    let before = free_at(&pool, ana, epoch(&pool, "2026-10-25T02:59:59Z").await).await;
    assert_eq!(before.left_micros, 1_000_000);
    assert_eq!(
        before.renews_at,
        epoch(&pool, "2026-10-25T03:00:00Z").await,
        "the next midnight in Brasília"
    );
    let after = free_at(&pool, ana, epoch(&pool, "2026-10-25T03:00:00Z").await).await;
    assert_eq!(after.left_micros, 4_000_000);
}

/// The month turns on the 1st, at midnight in Brasília; until then its cap
/// holds whatever the day would allow.
#[sqlx::test]
async fn the_first_of_the_month_renews_the_month(pool: PgPool) {
    allowance::set(&pool, FOUR_A_DAY).await.unwrap();
    let ana = account(&pool, "ana@example.com").await;
    for day in ["2026-10-02", "2026-10-09", "2026-10-16"] {
        spent_free_at(&pool, ana, &format!("{day}T15:00:00Z"), 4_000_000).await;
    }
    // October's $12 is spent: nothing left on the 31st, late in Brasília.
    let late = free_at(&pool, ana, epoch(&pool, "2026-11-01T02:59:59Z").await).await;
    assert_eq!(late.left_micros, 0);
    assert_eq!(
        late.renews_at,
        epoch(&pool, "2026-11-01T03:00:00Z").await,
        "the 1st, at midnight in Brasília"
    );
    let november = free_at(&pool, ana, epoch(&pool, "2026-11-01T03:00:00Z").await).await;
    assert_eq!(november.left_micros, 4_000_000);
}

#[sqlx::test]
async fn the_month_caps_the_day(pool: PgPool) {
    allowance::set(&pool, FOUR_A_DAY).await.unwrap();
    let ana = account(&pool, "ana@example.com").await;
    for day in ["2026-10-02", "2026-10-09"] {
        spent_free_at(&pool, ana, &format!("{day}T15:00:00Z"), 4_000_000).await;
    }
    spent_free_at(&pool, ana, "2026-10-16T15:00:00Z", 2_500_000).await;
    let then = free_at(&pool, ana, epoch(&pool, "2026-10-20T15:00:00Z").await).await;
    assert_eq!(
        then.left_micros, 1_500_000,
        "a fresh day, but $1.50 of the month"
    );
    assert_eq!(then.renews_at, epoch(&pool, "2026-10-21T03:00:00Z").await);
}
