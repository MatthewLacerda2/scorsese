//! The quote, and starting a purchase.

use std::sync::Arc;

use sqlx::postgres::PgPool;

use super::common::{self, request};
use super::{FIFTY_AT_4_99, Fake, bought, member, serve};

#[sqlx::test]
async fn a_quote_shows_the_credits_before_paying(pool: PgPool) {
    let address = serve(&pool, Some(Arc::default()), Some("4.99")).await;
    let (_, bearer) = member(&pool, "ana@example.com").await;
    let quote = request(
        address,
        "GET",
        "/api/payments/quote?centavos=5000",
        &[&bearer],
        None,
    )
    .await;
    assert_eq!(quote.status, 200, "{}", quote.body);
    let quote = quote.json();
    assert_eq!(quote["brl_centavos"], 5_000);
    assert_eq!(quote["credit_micros"], FIFTY_AT_4_99);
    assert_eq!(quote["brl_per_usd"], "4.99");
    assert_eq!(quote["rate_source"], "awesomeapi");
    assert_eq!(quote["minimum_centavos"], 2_000);
    assert_eq!(quote["maximum_centavos"], 500_000);

    let most = request(
        address,
        "GET",
        "/api/payments/quote?centavos=500000",
        &[&bearer],
        None,
    )
    .await;
    assert_eq!(most.status, 200, "{}", most.body);
    for centavos in [1_999, 500_001, 0, -5] {
        let path = format!("/api/payments/quote?centavos={centavos}");
        let refused = request(address, "GET", &path, &[&bearer], None).await;
        assert_eq!(refused.status, 400, "{centavos}: {}", refused.body);
    }
    assert_eq!(
        common::get(address, "/api/payments/quote?centavos=5000")
            .await
            .0,
        401
    );
}

#[sqlx::test]
async fn a_purchase_locks_its_rate_and_sends_the_browser_to_stripe(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    let address = serve(&pool, Some(fake.clone()), Some("4.99")).await;
    let (ana, bearer) = member(&pool, "ana@example.com").await;
    let body = serde_json::json!({ "centavos": 5_000 });
    let started = request(address, "POST", "/api/payments", &[&bearer], Some(&body)).await;
    assert_eq!(started.status, 200, "{}", started.body);
    let started = started.json();
    assert_eq!(started["url"], "https://checkout.stripe.com/c/pay/1");
    assert_eq!(started["session"], "cs_test_1");

    let order = fake.created.lock().unwrap()[0].clone();
    assert_eq!(order.centavos, 5_000);
    assert_eq!(order.name, "scorsese: $10.02 em créditos");
    assert_eq!(
        order.success_url,
        "https://scorsese.example/credits/return?session={CHECKOUT_SESSION_ID}"
    );
    assert_eq!(order.reference, ana.get().to_string());

    let row: (String, i64, i64, String, String) = sqlx::query_as(
        "SELECT state, brl_centavos, credit_micros, brl_per_usd, rate_source FROM payments",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        row,
        (
            "open".into(),
            5_000,
            FIFTY_AT_4_99,
            "4.99".into(),
            "awesomeapi".into()
        )
    );
}

#[sqlx::test]
async fn a_failed_rate_lookup_creates_no_session(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    let address = serve(&pool, Some(fake.clone()), None).await;
    let (_, bearer) = member(&pool, "ana@example.com").await;
    let body = serde_json::json!({ "centavos": 5_000 });
    let refused = request(address, "POST", "/api/payments", &[&bearer], Some(&body)).await;
    assert_eq!(refused.status, 503, "{}", refused.body);
    assert!(
        refused.json()["error"]
            .as_str()
            .unwrap()
            .contains("nothing was charged")
    );
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
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM payments")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rows, 0);
}

#[sqlx::test]
async fn a_server_without_stripe_says_so(pool: PgPool) {
    let address = serve(&pool, None, Some("4.99")).await;
    let (_, bearer) = member(&pool, "ana@example.com").await;
    let quote = request(
        address,
        "GET",
        "/api/payments/quote?centavos=5000",
        &[&bearer],
        None,
    )
    .await;
    assert_eq!(quote.status, 503, "{}", quote.body);
    assert!(
        quote.json()["error"]
            .as_str()
            .unwrap()
            .contains("not configured")
    );
    let hook = common::send(address, "POST", "/api/payments/stripe/webhook", &[], b"{}").await;
    assert_eq!(hook.status, 503, "{}", hook.body);
}

#[sqlx::test]
async fn two_purchases_are_two_sessions(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    let address = serve(&pool, Some(fake.clone()), Some("5")).await;
    let (_, bearer) = member(&pool, "ana@example.com").await;
    assert_eq!(bought(address, &bearer, 2_000).await, "cs_test_1");
    assert_eq!(bought(address, &bearer, 500_000).await, "cs_test_2");
    assert_eq!(fake.created(), 2);
}
