//! A still ordered in a half-price batch (#947), by the queue: ordered once,
//! asked after until it answers, charged half — or nothing, when the batch
//! stops without drawing it.

use scorsese_core::GenerationState;
use scorsese_server::credits::ledger;
use scorsese_server::db::{self, UserId};
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::paying::{fund, token};
use super::vendors::{Mock, serve, settled};
use super::{call, document, member};

/// A project with one sketched still in it — and a line, when `spoken`.
fn sketched(spoken: bool) -> Value {
    let mut assets = vec![json!({ "id": "backdrop", "kind": "generated_image",
        "state": "sketch", "prompt": "A harbour at dusk, wide." })];
    if spoken {
        assets.push(
            json!({ "id": "vo", "kind": "generated_audio", "state": "sketch",
            "prompt": "Every city has a night editor.", "speech": { "voice_id": "v" } }),
        );
    }
    json!({ "assets": assets })
}

/// Quote `project` as a batch and confirm it: what the yes said.
async fn batched(address: std::net::SocketAddr, who: &str, project: i64) -> (String, String) {
    let ask = json!({ "project": project, "batch": true });
    let (quote, refused) = call(address, who, "generate", ask).await;
    assert!(!refused, "{quote}");
    let yes = json!({ "project": project, "batch": true, "confirm": token(&quote) });
    let (said, refused) = call(address, who, "generate", yes).await;
    assert!(!refused, "{said}");
    (quote, said)
}

/// `user`'s balance, in micro-dollars.
async fn balance(pool: &PgPool, user: UserId) -> i64 {
    let mut tx = db::scoped(pool, user).await.unwrap();
    let balance = ledger::balance(&mut tx).await.unwrap();
    tx.commit().await.unwrap();
    balance
}

#[sqlx::test]
async fn a_batched_still_is_ordered_once_and_lands_at_half_price(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let id = super::stored(&pool, ana, sketched(false)).await;
    fund(&pool, ana, 10).await;

    let (quote, said) = batched(address, &who, id).await;
    assert!(
        quote.contains("backdrop: $0.03 — a 2K 16:9 still in nano_banana_2.1, in a batch"),
        "{quote}"
    );
    assert!(said.contains("half-price batch"), "{said}");
    assert_eq!(settled(&state, ana).await, vec!["done"]);
    assert_eq!((mock.ordered(), mock.drawn()), (1, 0));

    let stored = document(&pool, ana, id).await.document;
    assert_eq!(stored.assets[0].state, Some(GenerationState::Generated));
    let (kind,): (String,) = sqlx::query_as("SELECT kind FROM jobs WHERE kind <> 'thumbnail'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(kind, "batch_still");
    let (batch, ticket, state_of, micros): (bool, Option<String>, String, i64) =
        sqlx::query_as("SELECT batch, ticket, state, estimated_cost_micros FROM image_generations")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(batch && ticket.is_some_and(|ticket| ticket.starts_with("batches/mock-")));
    assert_eq!((state_of.as_str(), micros), ("generated", 30_000));
    assert_eq!(
        balance(&pool, ana).await,
        10_000_000 - 33_000,
        "half, with 10%"
    );

    let (again, _) = call(address, &who, "generate", json!({ "project": id })).await;
    assert!(again.contains("Nothing to pay for"), "{again}");
}

#[sqlx::test]
async fn a_batch_that_stops_without_drawing_is_free(pool: PgPool) {
    let mock = Mock {
        stop: Some("the batch expired".into()),
        ..Mock::default()
    };
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let id = super::stored(&pool, ana, sketched(false)).await;
    fund(&pool, ana, 10).await;

    batched(address, &who, id).await;
    assert_eq!(settled(&state, ana).await, vec!["failed"]);
    assert_eq!(mock.ordered(), 1);
    assert_eq!(balance(&pool, ana).await, 10_000_000, "nothing was charged");
    let stored = document(&pool, ana, id).await.document;
    assert_eq!(stored.assets[0].state, Some(GenerationState::Sketch));
}

#[sqlx::test]
async fn a_batch_with_a_line_to_send_is_refused_naming_it(pool: PgPool) {
    let address = super::common::serve(pool.clone()).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let id = super::stored(&pool, ana, sketched(true)).await;
    let ask = json!({ "project": id, "batch": true });
    let (said, refused) = call(address, &who, "generate", ask).await;
    assert!(
        refused && said.contains("only stills") && said.contains("vo"),
        "{said}"
    );
}
