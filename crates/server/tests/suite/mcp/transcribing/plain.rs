//! What a user is told on the web (#1549): a recording too long refused in
//! the call, a Whisper refusal in plain words, a generated item for what it
//! is.

use serde_json::json;
use sqlx::postgres::PgPool;

use super::stopped::imported;
use crate::mcp::vendors::{Mock, serve, settled};
use crate::mcp::{call, member};

#[sqlx::test]
async fn a_recording_past_two_hours_is_refused_in_the_call(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let (id, asset, item) = imported(&pool, &state, ana, address, &who).await;
    sqlx::query("UPDATE library_items SET media = $2 WHERE id = $1")
        .bind(item)
        .bind(json!({ "duration_seconds": 3.0 * 3600.0 }))
        .execute(&pool)
        .await
        .expect("long");
    let ask = json!({ "project": id, "asset": asset });
    let (said, refused) = call(address, &who, "transcribe", ask).await;
    assert!(refused, "{said}");
    assert!(
        said.contains("lasts 180 min, longer than the 120 min"),
        "{said}"
    );
    assert!(settled(&state, ana).await.is_empty(), "nothing queued");
}

/// A Whisper answering `status` fails the job in the user's words: what the
/// server said, and how to rebuild it, are the operator's.
async fn refused_with(pool: PgPool, status: u16) {
    let mock = Mock {
        refuses: Some(status),
        ..Mock::default()
    };
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let (id, asset, _) = imported(&pool, &state, ana, address, &who).await;
    let (said, refused) = call(
        address,
        &who,
        "transcribe",
        json!({ "project": id, "asset": asset }),
    )
    .await;
    assert!(!refused, "{said}");
    assert_eq!(settled(&state, ana).await, vec!["failed"]);
    let (said, _) = call(address, &who, "jobs", json!({})).await;
    for operator in [
        "Dockerfile",
        "rebuild",
        "installed locally",
        "deepdml",
        "500",
    ] {
        assert!(!said.contains(operator), "{status}: {operator} in {said}");
    }
    assert!(said.contains("nothing was spent"), "{status}: {said}");
}

#[sqlx::test]
async fn a_missing_model_is_ours_to_fix(pool: PgPool) {
    refused_with(pool, 404).await;
}

#[sqlx::test]
async fn too_much_at_once_is_said_plainly(pool: PgPool) {
    refused_with(pool, 413).await;
}

#[sqlx::test]
async fn a_busy_server_is_said_plainly(pool: PgPool) {
    refused_with(pool, 503).await;
}

#[sqlx::test]
async fn a_server_error_is_ours_to_fix(pool: PgPool) {
    refused_with(pool, 500).await;
}

#[sqlx::test]
async fn a_generated_shot_is_refused_as_what_it_is(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let (id, asset, item) = imported(&pool, &state, ana, address, &who).await;
    sqlx::query("UPDATE library_items SET brief_hash = $2 WHERE id = $1")
        .bind(item)
        .bind("b".repeat(64))
        .execute(&pool)
        .await
        .expect("generated");
    let (said, refused) = call(
        address,
        &who,
        "transcribe",
        json!({ "project": id, "asset": asset }),
    )
    .await;
    assert!(refused, "{said}");
    assert!(!said.contains("word timings"), "{said}");
    assert!(said.contains("was generated"), "{said}");
}
