//! The monthly fee: owed from the first top-up, charged once a month, only
//! from money that is there.

use scorsese_server::credits::MONTHLY_FEE_MICROS;
use scorsese_server::credits::fees::sweep;
use scorsese_server::db::{self, UserId};
use sqlx::postgres::PgPool;

use super::{account, balance, fund};

/// A top-up of `dollars` recorded `days_ago` — written directly, since the
/// ledger only ever says *now* on its own.
async fn funded_ago(pool: &PgPool, user: UserId, dollars: i64, days_ago: i32) {
    let mut tx = db::privileged(pool).await.expect("the test setup holds");
    sqlx::query(
        "INSERT INTO credit_entries (user_id, kind, amount_micros, memo, created_at)
         VALUES ($1, 'top_up', $2, 'Top-up', now() - make_interval(days => $3))",
    )
    .bind(user.get())
    .bind(dollars * 1_000_000)
    .bind(days_ago)
    .execute(&mut *tx)
    .await
    .expect("the test setup holds");
    tx.commit().await.expect("the test setup holds");
}

/// The months `user` has been charged for, oldest first.
async fn charged_months(pool: &PgPool, user: UserId) -> Vec<String> {
    let mut tx = db::scoped(pool, user).await.expect("the test setup holds");
    sqlx::query_scalar(
        "SELECT fee_period::text FROM credit_entries WHERE kind = 'monthly_fee' ORDER BY id",
    )
    .fetch_all(&mut *tx)
    .await
    .expect("the test setup holds")
}

#[sqlx::test]
async fn an_account_nobody_has_paid_for_owes_nothing(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    assert!(sweep(&pool).await.unwrap().is_empty());
    assert_eq!(balance(&pool, ana).await, 0);
}

#[sqlx::test]
async fn the_first_month_is_charged_when_the_first_money_arrives_and_only_once(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    fund(&pool, ana, 25).await;
    let charged = sweep(&pool).await.unwrap();
    assert_eq!(charged.len(), 1);
    assert_eq!(charged[0].0, ana);
    assert!(
        sweep(&pool).await.unwrap().is_empty(),
        "the same month twice"
    );
    assert_eq!(balance(&pool, ana).await, 25_000_000 - MONTHLY_FEE_MICROS);
    assert_eq!(charged_months(&pool, ana).await.len(), 1);
}

#[sqlx::test]
async fn a_fee_the_balance_does_not_cover_waits_for_a_top_up(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    fund(&pool, ana, 4).await;
    assert!(sweep(&pool).await.unwrap().is_empty());
    assert_eq!(
        balance(&pool, ana).await,
        4_000_000,
        "the ledger never lends"
    );

    fund(&pool, ana, 6).await;
    assert_eq!(sweep(&pool).await.unwrap().len(), 1);
    assert_eq!(balance(&pool, ana).await, 0);
}

/// Months are counted from the first top-up. Forty days on is the second
/// month; the first went uncovered by any sweep, and is not charged late.
#[sqlx::test]
async fn months_are_counted_from_the_first_top_up(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    let bia = account(&pool, "bia@example.com").await;
    funded_ago(&pool, ana, 30, 40).await;
    funded_ago(&pool, bia, 30, 5).await;
    sweep(&pool).await.unwrap();

    let mut tx = db::privileged(&pool).await.unwrap();
    let (second, first): (String, String) = sqlx::query_as(
        "SELECT ((now() AT TIME ZONE 'UTC' - interval '40 days') + interval '1 month')::date::text,
                (now() AT TIME ZONE 'UTC' - interval '5 days')::date::text",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(charged_months(&pool, ana).await, [second]);
    assert_eq!(charged_months(&pool, bia).await, [first]);
}
