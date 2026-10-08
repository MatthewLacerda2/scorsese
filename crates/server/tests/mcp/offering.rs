//! What a quote for now says about the half-price batch (#947): the batch
//! offered beside it when stills are worth a dollar, with a token of its own,
//! and a still waiting in a batch ordered elsewhere said plainly.

use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::{call, common, member};

/// A project with `count` sketched stills, each a different prompt.
fn stills(count: usize) -> Value {
    let assets: Vec<Value> = (0..count)
        .map(|n| {
            json!({ "id": format!("still{n}"), "kind": "generated_image", "state": "sketch",
                    "prompt": format!("A harbour at dusk, take {n}.") })
        })
        .collect();
    json!({ "assets": assets })
}

/// How many quotes are waiting to be spent.
async fn held(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM quotes")
        .fetch_one(pool)
        .await
        .expect("the quotes can be counted")
}

#[sqlx::test]
async fn a_dollar_of_stills_is_offered_as_a_batch_with_its_own_token(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let id = super::stored(&pool, ana, stills(20)).await;
    let (quote, refused) = call(address, &who, "generate", json!({ "project": id })).await;
    assert!(!refused, "{quote}");
    assert!(
        quote.contains("Stills: $1.32 now, or $0.66 in a half-price batch"),
        "{quote}"
    );
    assert_eq!(quote.matches("confirm: \"").count(), 1, "one token said");
    assert_eq!(held(&pool).await, 2, "and the batch's held beside it");
}

#[sqlx::test]
async fn pennies_of_stills_offer_nothing(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let id = super::stored(&pool, ana, stills(2)).await;
    let (quote, _) = call(address, &who, "generate", json!({ "project": id })).await;
    assert!(!quote.contains("half-price batch"), "{quote}");
    assert_eq!(held(&pool).await, 1);
}

#[sqlx::test]
async fn a_still_waiting_in_a_batch_ordered_elsewhere_is_said_plainly(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let waiting = json!({ "assets": [
        { "id": "backdrop", "kind": "generated_image", "state": "queued",
          "prompt": "A harbour at dusk.", "operation": "batches/on-a-laptop",
          "queued_at": "2026-10-08T12:00:00Z" }
    ] });
    let id = super::stored(&pool, ana, waiting).await;
    let (quote, _) = call(address, &who, "generate", json!({ "project": id })).await;
    assert!(
        quote.contains("backdrop: waiting in a batch ordered outside this server"),
        "{quote}"
    );
}
