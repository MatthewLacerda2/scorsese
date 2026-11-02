//! A paid purchase, and Stripe telling the webhook what later happened to
//! its charge: a refund or a dispute (#1637).

use std::net::SocketAddr;
use std::sync::Arc;

use scorsese_server::credits::ledger::{self, Link};
use scorsese_server::db::{self, UserId};
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::webhook::{deliver, event, now};
use super::{Fake, SECRET, bought, member, serve};

/// The payment intent behind the purchase [`fifty_paid`] makes.
pub(super) const INTENT: &str = "pi_test_1";

/// R$ 50 bought and paid by a fresh `ana` through the webhook, its session
/// naming [`INTENT`] — or naming none, `stored_before`, as a payment
/// credited before intents were kept.
pub(super) async fn fifty_paid(pool: &PgPool, stored_before: bool) -> (SocketAddr, UserId) {
    let fake = Arc::new(Fake::default());
    let address = serve(pool, Some(fake.clone()), Some("4.99")).await;
    let (ana, bearer) = member(pool, "ana@example.com").await;
    let session = bought(address, &bearer, 5_000).await;
    let mut paid = serde_json::to_value(fake.pay(&session)).expect("a session is JSON");
    paid["payment_intent"] = if stored_before {
        Value::Null
    } else {
        INTENT.into()
    };
    let completed = event("checkout.session.completed", &paid);
    let answered = deliver(address, &completed, SECRET, now()).await;
    assert_eq!(answered.status, 200, "{}", answered.body);
    (address, ana)
}

/// Deliver event `id` of `kind` about `object`, signed, and expect `200`.
pub(super) async fn tell(address: SocketAddr, id: &str, kind: &str, object: Value) {
    let body = json!({ "id": id, "object": "event", "type": kind, "data": { "object": object } });
    let body = body.to_string().into_bytes();
    let answered = deliver(address, &body, SECRET, now()).await;
    assert_eq!(answered.status, 200, "{kind}: {}", answered.body);
}

/// Event `id`: the charge behind `intent` has `refunded` centavos of its
/// R$ 50 refunded so far, from Stripe's Dashboard.
pub(super) async fn refunded(address: SocketAddr, id: &str, intent: &str, refunded: i64) {
    let charge = json!({
        "id": "ch_test_1", "object": "charge", "amount": 5_000,
        "amount_refunded": refunded, "currency": "brl",
        "payment_intent": intent, "refunded": refunded == 5_000,
    });
    tell(address, id, "charge.refunded", charge).await;
}

/// Event `id` of `kind` about the dispute on [`INTENT`]'s charge, now `status`.
pub(super) async fn disputed(address: SocketAddr, id: &str, kind: &str, status: &str) {
    let dispute = json!({
        "id": "dp_test_1", "object": "dispute", "amount": 5_000,
        "charge": "ch_test_1", "currency": "brl", "payment_intent": INTENT,
        "reason": "fraudulent", "status": status,
    });
    tell(address, id, kind, dispute).await;
}

/// Spend `micros` of `user`'s paid balance on a still that worked.
pub(super) async fn spend(pool: &PgPool, user: UserId, micros: i64) {
    let mut tx = db::scoped(pool, user).await.expect("a scope opens");
    let held = ledger::reserve(&mut tx, micros, "a still", Link::default()).await;
    let held = held.expect("the balance covers it");
    ledger::charge(&mut tx, held, micros)
        .await
        .expect("it charges");
    tx.commit().await.expect("it commits");
}
