//! What `transcribe` refuses, and a Whisper that is down (#1549).

use serde_json::json;
use sqlx::postgres::PgPool;

use super::footage;
use crate::vendors::{Mock, serve, settled};
use crate::{call, document, member};

#[sqlx::test]
async fn what_is_not_a_recording_is_refused_and_queues_nothing(pool: PgPool) {
    let mock = Mock::default();
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let id = crate::stored(
        &pool,
        ana,
        json!({ "assets": [{ "id": "title", "kind": "text", "text": "Hi" }] }),
    )
    .await;
    for (asked, says) in [
        (
            json!({ "project": id, "asset": "title" }),
            "`title` is text",
        ),
        (
            json!({ "project": id, "asset": "nope" }),
            "there is no asset `nope`",
        ),
        (json!({ "project": id }), "`asset` is required"),
        (
            json!({ "project": id, "asset": "title", "language": "Portuguese" }),
            "like `pt`",
        ),
    ] {
        let (said, refused) = call(address, &who, "transcribe", asked).await;
        assert!(refused, "{said}");
        assert!(said.contains(says), "{says} not in {said}");
    }
    let fix = json!({ "project": id, "asset": "title", "fixes": [{ "word": 1, "text": "x" }] });
    let (said, refused) = call(address, &who, "transcript_fix", fix).await;
    assert!(refused && said.contains("`title` is text"), "{said}");
    assert!(settled(&state, ana).await.is_empty());
}

#[sqlx::test]
async fn a_whisper_that_is_down_fails_the_job_in_words_for_the_user(pool: PgPool) {
    let mock = Mock {
        deaf: true,
        ..Mock::default()
    };
    let (address, state, _worker) = serve(&pool, &mock).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let (id, sha) = footage(&pool, &state, ana).await;
    let item: i64 = sqlx::query_scalar("SELECT id FROM library_items WHERE sha256 = $1")
        .bind(&sha)
        .fetch_one(&pool)
        .await
        .expect("held");
    call(
        address,
        &who,
        "import",
        json!({ "project": id, "items": [item] }),
    )
    .await;
    let asset = document(&pool, ana, id).await.document.assets[0]
        .id
        .to_string();
    let ask = json!({ "project": id, "asset": asset });
    let (said, refused) = call(address, &who, "transcribe", ask).await;
    assert!(!refused, "{said}");
    assert_eq!(settled(&state, ana).await, vec!["failed"]);
    let (said, _) = call(address, &who, "jobs", json!({})).await;
    assert!(
        said.contains("not answering") && said.contains("ours to fix"),
        "{said}"
    );
    assert!(
        !said.contains("docker"),
        "the operator's runbook is not the user's: {said}"
    );
}
