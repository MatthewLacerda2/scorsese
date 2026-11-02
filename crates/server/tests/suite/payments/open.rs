//! One open Checkout Session per member (#1654): a purchase started while
//! one is open is that one, and both routes that reach Stripe are limited.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use sqlx::postgres::PgPool;

use super::common::request;
use super::{FIFTY_AT_4_99, Fake, balance, bought, member, serve};

#[sqlx::test]
async fn a_second_purchase_while_one_is_open_is_that_one(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    let address = serve(&pool, Some(fake.clone()), Some("5")).await;
    let (_, bearer) = member(&pool, "ana@example.com").await;
    assert_eq!(bought(address, &bearer, 5_000).await, "cs_test_1");
    assert_eq!(bought(address, &bearer, 5_000).await, "cs_test_1");
    assert_eq!(fake.created(), 1, "one Checkout Session, not two");
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM payments")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rows, 1);
}

#[sqlx::test]
async fn a_paid_session_is_credited_and_the_next_purchase_is_new(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    let address = serve(&pool, Some(fake.clone()), Some("4.99")).await;
    let (ana, bearer) = member(&pool, "ana@example.com").await;
    let first = bought(address, &bearer, 5_000).await;
    fake.pay(&first);
    assert_eq!(bought(address, &bearer, 5_000).await, "cs_test_2");
    assert_eq!(
        balance(&pool, ana).await,
        FIFTY_AT_4_99,
        "settled on the way"
    );
}

#[sqlx::test]
async fn two_purchases_racing_open_one_session(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    let address = serve(&pool, Some(fake.clone()), Some("5")).await;
    let (_, bearer) = member(&pool, "ana@example.com").await;
    let (a, b) = tokio::join!(
        bought(address, &bearer, 5_000),
        bought(address, &bearer, 5_000)
    );
    assert_eq!(a, b);
    assert_eq!(fake.created(), 1);
}

#[sqlx::test]
async fn starting_and_confirming_are_limited_per_member(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    let address = serve(&pool, Some(fake.clone()), Some("5")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let (_, bia) = member(&pool, "bia@example.com").await;
    let body = serde_json::json!({ "centavos": 5_000 });
    for _ in 0..5 {
        bought(address, &ana, 5_000).await;
    }
    let refused = request(address, "POST", "/api/payments", &[&ana], Some(&body)).await;
    assert_eq!(refused.status, 429, "{}", refused.body);
    assert!(refused.header("retry-after").is_some());
    let session = bought(address, &bia, 5_000).await;

    let path = format!("/api/payments/{session}/confirm");
    for _ in 0..60 {
        let open = request(address, "POST", &path, &[&bia], None).await;
        assert_eq!(open.status, 200, "{}", open.body);
    }
    let refused = request(address, "POST", &path, &[&bia], None).await;
    assert_eq!(refused.status, 429, "{}", refused.body);
}

#[sqlx::test]
async fn a_session_paid_while_it_is_being_expired_is_credited(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    let address = serve(&pool, Some(fake.clone()), Some("4.99")).await;
    let (ana, bearer) = member(&pool, "ana@example.com").await;
    bought(address, &bearer, 5_000).await;
    fake.paid_before_expire.store(true, Ordering::SeqCst);
    assert_eq!(bought(address, &bearer, 6_000).await, "cs_test_2");
    assert_eq!(balance(&pool, ana).await, FIFTY_AT_4_99);
}

#[sqlx::test]
async fn an_open_session_near_its_end_is_not_handed_back(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    let address = serve(&pool, Some(fake.clone()), Some("5")).await;
    let (_, bearer) = member(&pool, "ana@example.com").await;
    bought(address, &bearer, 5_000).await;
    sqlx::query("UPDATE payments SET created_at = now() - interval '11 minutes'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(bought(address, &bearer, 5_000).await, "cs_test_2");
    let states: Vec<String> = sqlx::query_scalar("SELECT state FROM payments ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(states, ["expired", "open"]);
}

#[sqlx::test]
async fn an_open_session_stripe_does_not_know_holds_nothing_up(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    let address = serve(&pool, Some(fake.clone()), Some("5")).await;
    let (_, bearer) = member(&pool, "ana@example.com").await;
    bought(address, &bearer, 5_000).await;
    fake.forget();
    assert_eq!(bought(address, &bearer, 5_000).await, "cs_test_2");
}
