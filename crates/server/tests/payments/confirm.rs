//! `POST /api/payments/{session}/confirm`: the return page's way in, racing
//! the webhook.

use std::sync::Arc;

use scorsese_server::payments::settle;
use sqlx::postgres::PgPool;

use super::common::request;
use super::{FIFTY_AT_4_99, Fake, balance, bought, member, serve, top_ups};

#[sqlx::test]
async fn the_return_page_credits_a_paid_session_the_webhook_has_not(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    let address = serve(&pool, Some(fake.clone()), Some("4.99")).await;
    let (ana, bearer) = member(&pool, "ana@example.com").await;
    let session = bought(address, &bearer, 5_000).await;
    let path = format!("/api/payments/{session}/confirm");

    let open = request(address, "POST", &path, &[&bearer], None).await;
    assert_eq!(open.status, 200, "{}", open.body);
    assert_eq!(open.json()["state"], "open");
    assert_eq!(balance(&pool, ana).await, 0);

    fake.pay(&session);
    let paid = request(address, "POST", &path, &[&bearer], None).await;
    assert_eq!(paid.status, 200, "{}", paid.body);
    let paid = paid.json();
    assert_eq!(paid["state"], "paid");
    assert_eq!(paid["brl_centavos"], 5_000);
    assert_eq!(paid["credit_micros"], FIFTY_AT_4_99);
    assert_eq!(balance(&pool, ana).await, FIFTY_AT_4_99);

    let again = request(address, "POST", &path, &[&bearer], None).await;
    assert_eq!(again.json()["state"], "paid");
    assert_eq!(top_ups(&pool, ana).await, 1);
}

#[sqlx::test]
async fn the_webhook_and_the_return_page_racing_credit_once(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    let address = serve(&pool, Some(fake.clone()), Some("4.99")).await;
    let (ana, bearer) = member(&pool, "ana@example.com").await;
    for round in 0..8 {
        let session = bought(address, &bearer, 5_000).await;
        let paid = fake.pay(&session);
        let path = format!("/api/payments/{session}/confirm");
        let headers = [bearer.as_str()];
        let (hook, page, also) = tokio::join!(
            settle::settle(&pool, &paid),
            request(address, "POST", &path, &headers, None),
            settle::settle(&pool, &paid),
        );
        hook.unwrap();
        also.unwrap();
        assert_eq!(page.status, 200, "{}", page.body);
        assert_eq!(top_ups(&pool, ana).await, round + 1, "round {round}");
    }
    assert_eq!(balance(&pool, ana).await, 8 * FIFTY_AT_4_99);
}

#[sqlx::test]
async fn another_members_session_is_not_found(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    let address = serve(&pool, Some(fake.clone()), Some("4.99")).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let (bia, bearer) = member(&pool, "bia@example.com").await;
    let session = bought(address, &ana, 5_000).await;
    fake.pay(&session);

    let path = format!("/api/payments/{session}/confirm");
    let stranger = request(address, "POST", &path, &[&bearer], None).await;
    assert_eq!(stranger.status, 404, "{}", stranger.body);
    let nothing = request(
        address,
        "POST",
        "/api/payments/cs_none/confirm",
        &[&bearer],
        None,
    )
    .await;
    assert_eq!(nothing.status, 404, "{}", nothing.body);
    assert_eq!(balance(&pool, bia).await, 0);
}
