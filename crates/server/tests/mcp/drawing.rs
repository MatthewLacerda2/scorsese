//! A confirmed still (#461), drawn by the queue: kept in the library as an
//! image, charged, brought into the project and measured as a still.

use scorsese_core::GenerationState;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::paying::{fund, quote_and_confirm};
use super::vendors::{Mock, serve, settled};
use super::{call, document, member};

/// A project with one sketched still in it.
fn drawn() -> Value {
    json!({ "assets": [
        { "id": "backdrop", "kind": "generated_image", "state": "sketch",
          "prompt": "A harbour at dusk, wide.", "note": "the opening backdrop" }
    ] })
}

#[sqlx::test]
async fn a_still_lands_in_the_library_and_the_project(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let id = super::stored(&pool, ana, drawn()).await;
    fund(&pool, ana, 10).await;

    let (quote, _) = call(address, &who, "generate", json!({ "project": id })).await;
    assert!(
        quote.contains("backdrop: $0.11 — a 2K 16:9 still in flash"),
        "{quote}"
    );

    let (said, refused) = quote_and_confirm(address, &who, id).await;
    assert!(!refused, "{said}");
    assert_eq!(settled(&state, ana).await, vec!["done"]);
    assert_eq!(mock.drawn(), 1);

    let stored = document(&pool, ana, id).await.document;
    let still = &stored.assets[0];
    assert_eq!(still.state, Some(GenerationState::Generated));
    let path = still.path.as_ref().expect("a path").as_str();
    assert!(
        path.starts_with("generated/backdrop-") && path.ends_with(".jpg"),
        "{path}"
    );
    let media = still.media.expect("measured");
    assert_eq!((media.width, media.height), (Some(64), Some(36)));
    assert_eq!(
        media.duration_seconds, None,
        "a still has no length of its own"
    );

    let (kind, item, state_of): (String, Option<i64>, String) = sqlx::query_as(
        "SELECT l.kind, g.library_item_id, g.state FROM image_generations g
         JOIN library_items l ON l.id = g.library_item_id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((kind.as_str(), state_of.as_str()), ("image", "generated"));
    assert!(item.is_some());

    let (again, _) = call(address, &who, "generate", json!({ "project": id })).await;
    assert!(again.contains("Nothing to pay for"), "{again}");
    assert_eq!(mock.drawn(), 1, "never drawn twice");
}
