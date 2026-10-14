//! `POST /api/payments/stripe/webhook`: signed, idempotent, and the only
//! way in that needs no member.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use scorsese_providers::credentials::Secret;
use scorsese_providers::payments::webhook::sign;
use sqlx::postgres::PgPool;

use super::common::{self, Response};
use super::{FIFTY_AT_4_99, Fake, SECRET, balance, bought, member, serve, top_ups};

/// A delivery of `kind` about `session`, as Stripe would send it.
fn event(kind: &str, session: &serde_json::Value) -> Vec<u8> {
    serde_json::json!({
        "id": "evt_1", "object": "event", "type": kind,
        "data": { "object": session },
    })
    .to_string()
    .into_bytes()
}

/// POST `body` to the webhook, signed at `at` (seconds) under `secret`.
async fn deliver(address: SocketAddr, body: &[u8], secret: &str, at: i64) -> Response {
    let signature = sign(&Secret::new(secret), at, body);
    let header = format!("Stripe-Signature: t={at},v1={signature}");
    common::send(
        address,
        "POST",
        "/api/payments/stripe/webhook",
        &[&header],
        body,
    )
    .await
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[sqlx::test]
async fn the_same_event_twice_credits_once(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    let address = serve(&pool, Some(fake.clone()), Some("4.99")).await;
    let (ana, bearer) = member(&pool, "ana@example.com").await;
    let session = bought(address, &bearer, 5_000).await;
    let paid = serde_json::to_value(fake.pay(&session)).unwrap();

    let body = event("checkout.session.completed", &paid);
    for _ in 0..2 {
        let answered = deliver(address, &body, SECRET, now()).await;
        assert_eq!(answered.status, 200, "{}", answered.body);
    }
    // A delayed method's success after the completion changes nothing more.
    let late = event("checkout.session.async_payment_succeeded", &paid);
    assert_eq!(deliver(address, &late, SECRET, now()).await.status, 200);

    assert_eq!(balance(&pool, ana).await, FIFTY_AT_4_99);
    assert_eq!(top_ups(&pool, ana).await, 1);
    let (state, paid_at): (String, bool) =
        sqlx::query_as("SELECT state, paid_at IS NOT NULL FROM payments")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((state.as_str(), paid_at), ("paid", true));
}

#[sqlx::test]
async fn an_unsigned_tampered_or_stale_delivery_credits_nothing(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    let address = serve(&pool, Some(fake.clone()), Some("4.99")).await;
    let (ana, bearer) = member(&pool, "ana@example.com").await;
    let session = bought(address, &bearer, 5_000).await;
    let body = event(
        "checkout.session.completed",
        &serde_json::to_value(fake.pay(&session)).unwrap(),
    );

    let unsigned = common::send(address, "POST", "/api/payments/stripe/webhook", &[], &body).await;
    assert_eq!(unsigned.status, 400, "{}", unsigned.body);
    assert_eq!(
        deliver(address, &body, "whsec_wrong", now()).await.status,
        400
    );
    assert_eq!(
        deliver(address, &body, SECRET, now() - 3_600).await.status,
        400
    );
    let signature = sign(&Secret::new(SECRET), now(), &body);
    let mut tampered = body.clone();
    let at = tampered.len() - 3;
    tampered[at] = b' ';
    let header = format!("Stripe-Signature: t={},v1={signature}", now());
    let forged = common::send(
        address,
        "POST",
        "/api/payments/stripe/webhook",
        &[&header],
        &tampered,
    )
    .await;
    assert_eq!(forged.status, 400, "{}", forged.body);

    assert_eq!(balance(&pool, ana).await, 0);
}

#[sqlx::test]
async fn an_expired_session_credits_nothing(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    let address = serve(&pool, Some(fake.clone()), Some("4.99")).await;
    let (ana, bearer) = member(&pool, "ana@example.com").await;
    let session = bought(address, &bearer, 5_000).await;
    let expired = serde_json::json!({
        "id": session, "status": "expired", "payment_status": "unpaid",
        "amount_total": 5_000, "currency": "brl",
    });
    let answered = deliver(
        address,
        &event("checkout.session.expired", &expired),
        SECRET,
        now(),
    )
    .await;
    assert_eq!(answered.status, 200, "{}", answered.body);

    assert_eq!(balance(&pool, ana).await, 0);
    let state: String = sqlx::query_scalar("SELECT state FROM payments")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "expired");
}

#[sqlx::test]
async fn a_paid_amount_that_disagrees_or_an_unknown_session_credits_nothing(pool: PgPool) {
    let fake = Arc::new(Fake::default());
    let address = serve(&pool, Some(fake.clone()), Some("4.99")).await;
    let (ana, bearer) = member(&pool, "ana@example.com").await;
    let session = bought(address, &bearer, 5_000).await;
    let mut paid = serde_json::to_value(fake.pay(&session)).unwrap();
    paid["amount_total"] = 100.into();
    let body = event("checkout.session.completed", &paid);
    assert_eq!(deliver(address, &body, SECRET, now()).await.status, 200);

    paid["id"] = "cs_test_elsewhere".into();
    let body = event("checkout.session.completed", &paid);
    assert_eq!(deliver(address, &body, SECRET, now()).await.status, 200);
    let other = event("charge.refunded", &serde_json::json!({ "id": "ch_1" }));
    assert_eq!(deliver(address, &other, SECRET, now()).await.status, 200);

    assert_eq!(balance(&pool, ana).await, 0);
}
