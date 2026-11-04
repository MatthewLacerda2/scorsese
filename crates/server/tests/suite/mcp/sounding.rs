//! A generated sound effect on the web (#1265): quoted by its length, paid at
//! ×1.8, made by the queue, kept in the library, audited — never paid for
//! twice, and free when ElevenLabs fails.

use std::sync::atomic::Ordering;

use scorsese_core::GenerationState;
use scorsese_server::credits::ledger;
use scorsese_server::db::{self, UserId};
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::paying::{fund, quote_and_confirm};
use super::vendors::{Mock, serve, settled};
use super::{call, document, member};

/// A project of one two-and-a-half-second sound, made to loop.
fn foley() -> Value {
    json!({ "assets": [
        { "id": "rain", "kind": "generated_audio", "state": "sketch",
          "prompt": "Light rain on a window.",
          "sound": { "seconds": 2.5, "loop": true } }
    ] })
}

async fn balance(pool: &PgPool, user: UserId) -> i64 {
    let mut tx = db::scoped(pool, user).await.expect("a scoped transaction");
    let balance = ledger::balance(&mut tx).await.expect("the balance");
    tx.commit().await.expect("the commit");
    balance
}

#[sqlx::test]
async fn a_sound_is_quoted_made_and_charged(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let id = super::stored(&pool, ana, foley()).await;
    fund(&pool, ana, 10).await;

    let (quote, _) = call(address, &who, "generate", json!({ "project": id })).await;
    assert!(
        quote.contains("rain: $0.01 — 2.5 seconds of sound"),
        "{quote}"
    );
    let (said, refused) = quote_and_confirm(address, &who, id).await;
    assert!(!refused, "{said}");
    assert_eq!(settled(&state, ana).await, vec!["done"]);
    assert_eq!(mock.sounded.load(Ordering::SeqCst), 1);
    assert_eq!(mock.composed.load(Ordering::SeqCst), 0, "never composed");
    assert_eq!(mock.spoken(), 0, "and never spoken");

    let stored = document(&pool, ana, id).await.document;
    let sound = &stored.assets[0];
    assert_eq!(sound.state, Some(GenerationState::Generated));
    let path = sound.path.as_ref().expect("a path").as_str().to_owned();
    assert!(path.starts_with("generated/rain-"), "{path}");

    let (item, state_of, millis, looping): (Option<i64>, String, i32, bool) =
        sqlx::query_as("SELECT library_item_id, state, millis, loop FROM sound_generations")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(item.is_some());
    assert_eq!(
        (state_of.as_str(), millis, looping),
        ("generated", 2500, true)
    );
    assert_eq!(
        balance(&pool, ana).await,
        10_000_000 - 18_000,
        "a cent at ×1.8"
    );

    let (history, _) = call(address, &who, "spending_history", json!({})).await;
    assert!(history.contains("Sound effect: 2.5s"), "{history}");

    // The same sound in another of ana's projects is already hers.
    let other = super::stored(&pool, ana, foley()).await;
    let (quote, _) = call(address, &who, "generate", json!({ "project": other })).await;
    assert!(!quote.contains("rain:"), "{quote}");
    assert!(quote.contains("1 already made"), "{quote}");
    assert_eq!(mock.sounded.load(Ordering::SeqCst), 1, "never made twice");
}

#[sqlx::test]
async fn a_sound_the_vendor_fails_is_free(pool: PgPool) {
    let mock = Mock {
        fail: Some("the sound service is down".into()),
        ..Mock::default()
    };
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let id = super::stored(&pool, ana, foley()).await;
    fund(&pool, ana, 10).await;

    let (said, refused) = quote_and_confirm(address, &who, id).await;
    assert!(!refused, "{said}");
    assert_eq!(settled(&state, ana).await, vec!["failed"]);
    assert_eq!(mock.sounded.load(Ordering::SeqCst), 1);
    let (state_of,): (String,) = sqlx::query_as("SELECT state FROM sound_generations")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state_of, "failed");
    assert_eq!(balance(&pool, ana).await, 10_000_000, "nothing was spent");
}
