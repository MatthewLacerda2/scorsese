//! A confirmed generation, made by the queue: kept in the library, charged,
//! brought into the project — and never paid for twice.

use scorsese_core::GenerationState;
use scorsese_server::credits::ledger;
use scorsese_server::db;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::paying::{fund, narrated, quote_and_confirm};
use super::vendors::{Mock, serve, settled};
use super::{call, document, member};

#[sqlx::test]
async fn a_spoken_line_lands_in_the_library_and_the_project(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let id = super::stored(&pool, ana, narrated()).await;
    fund(&pool, ana, 10).await;

    let (said, refused) = quote_and_confirm(address, &who, id).await;
    assert!(!refused, "{said}");
    assert_eq!(settled(&state, ana).await, vec!["done"]);
    assert_eq!(mock.spoken(), 1);

    let stored = document(&pool, ana, id).await.document;
    let line = &stored.assets[0];
    assert_eq!(line.state, Some(GenerationState::Generated));
    let path = line.path.as_ref().expect("a path").as_str().to_owned();
    assert!(
        path.starts_with("generated/vo-") && path.ends_with(".mp3"),
        "{path}"
    );
    let seconds = line.media.and_then(|media| media.duration_seconds);
    assert!(
        seconds.is_some_and(|seconds| seconds > 0.5),
        "measured: {seconds:?}"
    );

    let (item, state_of): (Option<i64>, String) =
        sqlx::query_as("SELECT library_item_id, state FROM speech_generations")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(item.is_some());
    assert_eq!(state_of, "generated");
    let mut tx = db::scoped(&pool, ana).await.unwrap();
    let balance = ledger::balance(&mut tx).await.unwrap();
    tx.commit().await.unwrap();
    assert!(balance < 10_000_000, "the line was charged: {balance}");

    // The same line in another of ana's projects is already hers.
    let other = super::stored(&pool, ana, narrated()).await;
    let (quote, _) = call(address, &who, "generate", json!({ "project": other })).await;
    assert!(quote.contains("vo: already spoken"), "{quote}");
    assert!(quote.contains("Nothing to pay for"), "{quote}");
    let adopted = document(&pool, ana, other).await.document;
    assert_eq!(adopted.assets[0].path, line.path, "brought in, free");
    assert_eq!(mock.spoken(), 1, "and never spoken twice");
}

/// Somebody else's generation of the very same brief is theirs: a second
/// user pays for their own.
#[sqlx::test]
async fn a_brief_is_never_shared_between_users(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, hers) = member(&pool, "ana@example.com").await;
    let (bob, his) = member(&pool, "bob@example.com").await;
    let (a, b) = (
        super::stored(&pool, ana, narrated()).await,
        super::stored(&pool, bob, narrated()).await,
    );
    fund(&pool, ana, 10).await;
    quote_and_confirm(address, &hers, a).await;
    assert_eq!(settled(&state, ana).await, vec!["done"]);
    let (quote, _) = call(address, &his, "generate", json!({ "project": b })).await;
    assert!(
        quote.contains("vo: $0.0"),
        "bob is quoted for his own: {quote}"
    );
}
