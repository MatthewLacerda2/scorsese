//! How long a message to the assistant may be (#1648): 32,000 characters,
//! counted as characters, and a body far past that is refused unread.

use serde_json::json;
use sqlx::postgres::PgPool;

use super::{Script, answers, common, finished, member, project, scripted, send};

#[sqlx::test]
async fn a_prompt_is_at_most_thirty_two_thousand_characters(pool: PgPool) {
    let script = Script::new(vec![answers("Done.")]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;

    // Two bytes each: the bound counts characters, not bytes.
    let sent = send(address, &who, id, &"é".repeat(32_000)).await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    finished(address, &who, sent.json()["id"].as_i64().unwrap()).await;

    let sent = send(address, &who, id, &"é".repeat(32_001)).await;
    assert_eq!(sent.status, 400, "{}", sent.body);
    assert!(sent.body.contains("32000 characters"), "{}", sent.body);
    let turns: i64 = sqlx::query_scalar("SELECT count(*) FROM chat_turns")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(turns, 1, "a refused prompt starts no turn");

    // A body far past any prompt is refused before it is read.
    let path = format!("/api/projects/{id}/chat");
    let body = json!({ "prompt": "x".repeat(1024 * 1024) });
    let huge = common::request(address, "POST", &path, &[who.as_str()], Some(&body)).await;
    assert_eq!(huge.status, 413, "{}", huge.body);
}
