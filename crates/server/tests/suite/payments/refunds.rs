//! A refund made in Stripe's Dashboard (#1637): it takes back what a
//! withdrawal would, in the share refunded, and never what was taken already.

use scorsese_server::credits::ledger;
use scorsese_server::db;
use scorsese_server::payments::withdraw;
use sqlx::postgres::PgPool;

use super::FIFTY_AT_4_99;
use super::bonus::entries;
use super::charges::{INTENT, disputed, fifty_paid, refunded, spend};
use super::{balance, top_ups};

#[sqlx::test]
async fn a_full_refund_takes_back_an_untouched_purchase_once(pool: PgPool) {
    let (address, ana) = fifty_paid(&pool, false).await;
    for _ in 0..2 {
        refunded(address, "evt_refund_1", INTENT, 5_000).await;
    }
    assert_eq!(balance(&pool, ana).await, 0);
    assert_eq!(entries(&pool, ana, "purchase_refunded").await, 1);
    assert_eq!(top_ups(&pool, ana).await, 1, "the top-up itself stays");
}

#[sqlx::test]
async fn a_half_refund_takes_half_of_what_is_left_and_the_rest_the_rest(pool: PgPool) {
    let (address, ana) = fifty_paid(&pool, false).await;
    spend(&pool, ana, 2_000_000).await;
    let left = FIFTY_AT_4_99 - 2_000_000;
    refunded(address, "evt_refund_1", INTENT, 2_500).await;
    assert_eq!(balance(&pool, ana).await, left - left / 2);
    // Delivered late, the first refund again takes nothing more.
    refunded(address, "evt_refund_2", INTENT, 5_000).await;
    refunded(address, "evt_refund_1", INTENT, 2_500).await;
    assert_eq!(balance(&pool, ana).await, 0);
    assert_eq!(entries(&pool, ana, "purchase_refunded").await, 2);
}

#[sqlx::test]
async fn a_refund_never_takes_past_zero(pool: PgPool) {
    let (address, ana) = fifty_paid(&pool, false).await;
    spend(&pool, ana, FIFTY_AT_4_99).await;
    refunded(address, "evt_refund_1", INTENT, 5_000).await;
    assert_eq!(balance(&pool, ana).await, 0);
}

#[sqlx::test]
async fn a_refund_the_withdrawal_already_took_takes_nothing_more(pool: PgPool) {
    let (address, ana) = fifty_paid(&pool, false).await;
    let session: String = sqlx::query_scalar("SELECT stripe_session FROM payments")
        .fetch_one(&pool)
        .await
        .unwrap();
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let withdrawn = withdraw::withdraw(&mut tx, &session, None).await.unwrap();
    ledger::top_up(&mut tx, 5_000_000).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(withdrawn.refund_centavos, 5_000);

    refunded(address, "evt_refund_1", INTENT, 5_000).await;
    assert_eq!(balance(&pool, ana).await, 5_000_000);
    assert_eq!(entries(&pool, ana, "purchase_refunded").await, 0);
}

#[sqlx::test]
async fn an_intent_this_server_does_not_know_is_answered_and_ignored(pool: PgPool) {
    let (address, ana) = fifty_paid(&pool, false).await;
    refunded(address, "evt_refund_1", "pi_elsewhere", 5_000).await;
    assert_eq!(balance(&pool, ana).await, FIFTY_AT_4_99);
}

#[sqlx::test]
async fn a_purchase_credited_before_intents_were_kept_is_found_by_its_session(pool: PgPool) {
    let (address, ana) = fifty_paid(&pool, true).await;
    refunded(address, "evt_refund_1", INTENT, 5_000).await;
    assert_eq!(balance(&pool, ana).await, 0);
}

#[sqlx::test]
async fn a_purchase_refunded_in_stripe_is_not_withdrawn_again(pool: PgPool) {
    let (address, ana) = fifty_paid(&pool, false).await;
    refunded(address, "evt_refund_1", INTENT, 2_500).await;
    let session: String = sqlx::query_scalar("SELECT stripe_session FROM payments")
        .fetch_one(&pool)
        .await
        .unwrap();
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let again = withdraw::withdraw(&mut tx, &session, None).await;
    let refused = again.expect_err("Stripe took it back already").to_string();
    assert!(
        refused.contains("refunded or disputed in Stripe"),
        "{refused}"
    );
}

#[sqlx::test]
async fn an_inquiry_closed_by_a_full_refund_gives_nothing_back(pool: PgPool) {
    let (address, ana) = fifty_paid(&pool, false).await;
    disputed(
        address,
        "evt_1",
        "charge.dispute.created",
        "warning_needs_response",
    )
    .await;
    refunded(address, "evt_2", INTENT, 5_000).await;
    disputed(address, "evt_3", "charge.dispute.closed", "warning_closed").await;
    assert_eq!(balance(&pool, ana).await, 0);
}

#[sqlx::test]
async fn a_dispute_won_after_a_half_refund_gives_back_half(pool: PgPool) {
    let (address, ana) = fifty_paid(&pool, false).await;
    disputed(address, "evt_1", "charge.dispute.created", "needs_response").await;
    refunded(address, "evt_2", INTENT, 2_500).await;
    disputed(address, "evt_3", "charge.dispute.closed", "won").await;
    assert_eq!(balance(&pool, ana).await, FIFTY_AT_4_99 / 2);
}
