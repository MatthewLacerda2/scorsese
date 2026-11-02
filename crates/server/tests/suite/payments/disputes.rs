//! A disputed purchase (#1637): the whole of it taken back, past zero; no
//! spending while the debt stands, free or paid; a won dispute undone.

use scorsese_server::credits::allowance::{self, Allowance};
use scorsese_server::credits::history::{self, Filter};
use scorsese_server::credits::ledger::{self, Link};
use scorsese_server::db;
use sqlx::postgres::PgPool;

use super::FIFTY_AT_4_99;
use super::balance;
use super::bonus::entries;
use super::charges::{disputed, fifty_paid, spend};

const OPENED: &str = "charge.dispute.created";
const CLOSED: &str = "charge.dispute.closed";

#[sqlx::test]
async fn a_dispute_on_a_half_spent_purchase_stops_every_spend_until_paid_off(pool: PgPool) {
    let (address, ana) = fifty_paid(&pool, false).await;
    let half = FIFTY_AT_4_99 / 2;
    spend(&pool, ana, half).await;
    disputed(address, "evt_dispute_1", OPENED, "needs_response").await;
    assert_eq!(balance(&pool, ana).await, -half);

    // Free credit pays for nothing while the debt stands.
    allowance::set(&pool, Allowance::DEFAULT).await.unwrap();
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    assert_eq!(ledger::spendable(&mut tx).await.unwrap(), -half);
    let refused = ledger::reserve(&mut tx, 1_000, "a line", Link::default()).await;
    let refused = refused
        .expect_err("a disputed account spends nothing")
        .to_string();
    assert!(
        refused.contains("because a payment was disputed"),
        "{refused}"
    );
    assert!(
        refused.contains("add credit to keep generating"),
        "{refused}"
    );
    drop(tx);

    // A top-up past the debt lets spending resume.
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    ledger::top_up(&mut tx, half + 1_000_000).await.unwrap();
    ledger::reserve(&mut tx, 1_000, "a line", Link::default())
        .await
        .expect("paid off, it spends again");
}

#[sqlx::test]
async fn a_dispute_takes_once_and_a_won_one_gives_back_exactly_that(pool: PgPool) {
    let (address, ana) = fifty_paid(&pool, false).await;
    disputed(address, "evt_dispute_1", OPENED, "needs_response").await;
    disputed(address, "evt_dispute_1", OPENED, "needs_response").await;
    assert_eq!(balance(&pool, ana).await, 0);
    assert_eq!(entries(&pool, ana, "dispute").await, 1);

    let read = history::read(&pool, ana, &Filter::default()).await.unwrap();
    let memo = &read.rows[0].memo;
    assert!(memo.starts_with("R$ 50,00 purchase of "), "{memo}");
    assert!(memo.contains("disputed with your card's bank"), "{memo}");

    for _ in 0..2 {
        disputed(address, "evt_dispute_2", CLOSED, "won").await;
    }
    assert_eq!(balance(&pool, ana).await, FIFTY_AT_4_99);
    assert_eq!(entries(&pool, ana, "dispute_won").await, 1);
    let read = history::read(&pool, ana, &Filter::default()).await.unwrap();
    assert!(
        read.rows[0]
            .memo
            .starts_with("dispute settled in your favour")
    );
}

#[sqlx::test]
async fn a_lost_dispute_changes_nothing_more(pool: PgPool) {
    let (address, ana) = fifty_paid(&pool, false).await;
    disputed(address, "evt_dispute_1", OPENED, "needs_response").await;
    disputed(address, "evt_dispute_2", CLOSED, "lost").await;
    assert_eq!(balance(&pool, ana).await, 0);
    assert_eq!(entries(&pool, ana, "dispute_won").await, 0);
}

#[sqlx::test]
async fn a_win_delivered_before_its_dispute_leaves_the_purchase_whole(pool: PgPool) {
    let (address, ana) = fifty_paid(&pool, false).await;
    disputed(address, "evt_dispute_2", CLOSED, "won").await;
    disputed(address, "evt_dispute_1", OPENED, "needs_response").await;
    assert_eq!(balance(&pool, ana).await, FIFTY_AT_4_99);
}
