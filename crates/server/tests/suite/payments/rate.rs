//! The rate is kept, not fetched per purchase: one older than ten minutes is
//! not sold at, and a purchase is made at the rate its buyer was shown.

use std::sync::Arc;
use std::time::{Duration, Instant};

use scorsese_providers::fx::{Rate, Source};
use sqlx::postgres::PgPool;

use super::common::request;
use super::{Fake, member, rates, serve_kept};

/// The payments rows written.
async fn payments(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM payments")
        .fetch_one(pool)
        .await
        .expect("the payments table reads")
}

#[sqlx::test]
async fn a_rate_older_than_ten_minutes_sells_nothing(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    // The source fails, so the rate kept is the old one and nothing newer.
    let latest = rates(None);
    let rate = Rate::parse("5.01", Source::Ptax).unwrap();
    let eleven_minutes_ago = Instant::now()
        .checked_sub(Duration::from_secs(11 * 60))
        .unwrap();
    latest.keep(rate, eleven_minutes_ago);
    let address = serve_kept(&pool, Some(fake.clone()), &latest).await;
    let (_, bearer) = member(&pool, "ana@example.com").await;

    let body = serde_json::json!({ "centavos": 5_000 });
    let refused = request(address, "POST", "/api/payments", &[&bearer], Some(&body)).await;
    assert_eq!(refused.status, 503, "{}", refused.body);
    let quote = request(
        address,
        "GET",
        "/api/payments/quote?centavos=5000",
        &[&bearer],
        None,
    )
    .await;
    assert_eq!(quote.status, 503, "{}", quote.body);
    assert_eq!(fake.created(), 0);
    assert_eq!(payments(&pool).await, 0);
}

#[sqlx::test]
async fn a_rate_that_moved_since_the_quote_is_quoted_again(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    let latest = rates(Some("5.01"));
    let address = serve_kept(&pool, Some(fake.clone()), &latest).await;
    let (_, bearer) = member(&pool, "ana@example.com").await;

    let stale = serde_json::json!({ "centavos": 5_000, "brl_per_usd": "4.99" });
    let moved = request(address, "POST", "/api/payments", &[&bearer], Some(&stale)).await;
    assert_eq!(moved.status, 409, "{}", moved.body);
    assert_eq!(moved.json()["quote"]["brl_per_usd"], "5.01");
    assert_eq!(fake.created(), 0);
    assert_eq!(payments(&pool).await, 0);

    let shown = serde_json::json!({ "centavos": 5_000, "brl_per_usd": "5.01" });
    let paid = request(address, "POST", "/api/payments", &[&bearer], Some(&shown)).await;
    assert_eq!(paid.status, 200, "{}", paid.body);
    let stored: String = sqlx::query_scalar("SELECT brl_per_usd FROM payments")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored, "5.01");
}
