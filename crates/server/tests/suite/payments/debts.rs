//! When a dispute's debt stops spending (#1637): from the dispute until the
//! paid balance is back to zero or above, and never for a dispute that left
//! it above zero.

use scorsese_server::credits::allowance::{self, Allowance};
use scorsese_server::credits::ledger::{self, Link};
use scorsese_server::db::{self, UserId};
use sqlx::postgres::PgPool;

use super::FIFTY_AT_4_99;
use super::charges::{disputed, fifty_paid, spend};

const OPENED: &str = "charge.dispute.created";
const CLOSED: &str = "charge.dispute.closed";

/// An assistant call that overran `user`'s paid balance by `micros`, as
/// `credits` allows; then a free line, which the allowance must pay.
async fn overrun_then_spend_free(pool: &PgPool, user: UserId, micros: i64) {
    let mut tx = db::scoped(pool, user).await.expect("the test setup holds");
    sqlx::query(
        "INSERT INTO credit_entries (user_id, kind, amount_micros, memo)
         VALUES (member_id(), 'charge', $1, 'Assistant: overran')",
    )
    .bind(-micros)
    .execute(&mut *tx)
    .await
    .expect("the test setup holds");
    tx.commit().await.expect("the test setup holds");
    allowance::set(pool, Allowance::DEFAULT)
        .await
        .expect("the test setup holds");
    let mut tx = db::scoped(pool, user).await.expect("the test setup holds");
    ledger::reserve(&mut tx, 1_000, "a line", Link::default())
        .await
        .expect("no debt stands; free credit pays");
}

#[sqlx::test]
async fn a_debt_paid_off_stops_nothing_the_allowance_pays_for_later(pool: PgPool) {
    let (address, ana) = fifty_paid(&pool, false).await;
    spend(&pool, ana, FIFTY_AT_4_99).await;
    disputed(address, "evt_dispute_1", OPENED, "needs_response").await;
    disputed(address, "evt_dispute_2", CLOSED, "lost").await;
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    ledger::top_up(&mut tx, FIFTY_AT_4_99).await.unwrap();
    tx.commit().await.unwrap();
    overrun_then_spend_free(&pool, ana, 10_000).await;
}

#[sqlx::test]
async fn a_dispute_that_left_the_balance_above_zero_owes_nothing(pool: PgPool) {
    let (address, ana) = fifty_paid(&pool, false).await;
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    ledger::top_up(&mut tx, 1_000_000).await.unwrap();
    tx.commit().await.unwrap();
    disputed(address, "evt_dispute_1", OPENED, "needs_response").await;
    overrun_then_spend_free(&pool, ana, 1_010_000).await;
}
