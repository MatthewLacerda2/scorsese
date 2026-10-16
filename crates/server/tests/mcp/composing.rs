//! A generated track on the web (#1264): quoted by its length, paid at ×1.8,
//! composed by the queue, kept in the library, audited — and never paid for
//! twice.

use std::sync::atomic::Ordering;

use scorsese_core::GenerationState;
use scorsese_server::credits::ledger;
use scorsese_server::db;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::paying::{fund, quote_and_confirm};
use super::vendors::{Mock, serve, settled};
use super::{call, document, member};

/// A project of one forty-second track.
fn scored() -> Value {
    json!({ "assets": [
        { "id": "bed", "kind": "generated_audio", "state": "sketch",
          "prompt": "Calm lo-fi, soft piano.", "music": { "seconds": 40 } }
    ] })
}

#[sqlx::test]
async fn a_track_is_quoted_composed_and_charged(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let id = super::stored(&pool, ana, scored()).await;
    fund(&pool, ana, 10).await;

    let (quote, _) = call(address, &who, "generate", json!({ "project": id })).await;
    assert!(
        quote.contains("bed: $0.10 — 40 seconds of music"),
        "{quote}"
    );
    let (said, refused) = quote_and_confirm(address, &who, id).await;
    assert!(!refused, "{said}");
    assert_eq!(settled(&state, ana).await, vec!["done"]);
    assert_eq!(mock.composed.load(Ordering::SeqCst), 1);
    assert_eq!(mock.spoken(), 0, "a track is never spoken");

    let stored = document(&pool, ana, id).await.document;
    let track = &stored.assets[0];
    assert_eq!(track.state, Some(GenerationState::Generated));
    let path = track.path.as_ref().expect("a path").as_str().to_owned();
    assert!(path.starts_with("generated/bed-"), "{path}");

    let (item, state_of, seconds): (Option<i64>, String, i32) =
        sqlx::query_as("SELECT library_item_id, state, seconds FROM music_generations")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(item.is_some());
    assert_eq!((state_of.as_str(), seconds), ("generated", 40));
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let balance = ledger::balance(&mut tx).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(balance, 10_000_000 - 180_000, "10 cents at ×1.8");

    let (history, _) = call(address, &who, "spending_history", json!({})).await;
    assert!(history.contains("Music track: 40s"), "{history}");

    // The same track in another of ana's projects is already hers.
    let other = super::stored(&pool, ana, scored()).await;
    let (quote, _) = call(address, &who, "generate", json!({ "project": other })).await;
    assert!(quote.contains("bed: already composed"), "{quote}");
    assert_eq!(
        mock.composed.load(Ordering::SeqCst),
        1,
        "and never composed twice"
    );
}
