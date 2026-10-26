//! Migration `0031` (#1220): a project still on a Gemini model, cut from the
//! assistant, opens its chat on Sonnet, and a question a Gemini turn was
//! waiting on is set aside rather than resumed in Gemini's wire.

use serde_json::json;
use sqlx::postgres::PgPool;

use super::{Script, answers, calls, common, finished, member, new_project, scripted, send};

/// The migration as it ships. It only rewrites rows naming a Gemini model, so
/// running it again over a database that already has it is what a database
/// from before it would go through.
const MIGRATION: &str = include_str!("../../../migrations/0031_assistant_without_gemini.sql");

#[sqlx::test]
async fn a_project_left_on_gemini_opens_on_sonnet_with_its_question_set_aside(pool: PgPool) {
    let question = json!({ "question": "Loop it or fade it?", "options": ["loop it", "fade it"] });
    let script = Script::new(vec![calls("ask_user", question)]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = new_project(&pool, ana, json!([])).await;
    let sent = send(address, &who, id, "add the music").await;
    let turn = sent.json()["id"].as_i64().unwrap();
    assert_eq!(
        finished(address, &who, turn).await["turn"]["state"],
        "asking"
    );
    // As 0019 left the project, and a turn paused while it ran on Gemini.
    sqlx::query("UPDATE projects SET assistant_model = 'gemini-3.8-flash' WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE chat_turns SET model = 'gemini-3.8-flash' WHERE id = $1")
        .bind(turn)
        .execute(&pool)
        .await
        .unwrap();
    let chat = format!("/api/projects/{id}/chat");
    let stranded = common::request(address, "GET", &chat, &[&who], None).await;
    assert_ne!(stranded.status, 200, "an unknown model: {}", stranded.body);

    sqlx::raw_sql(MIGRATION).execute(&pool).await.unwrap();

    let opened = common::request(address, "GET", &chat, &[&who], None).await;
    assert_eq!(opened.status, 200, "{}", opened.body);
    let opened = opened.json();
    assert_eq!(opened["model"], "claude-sonnet-5-5");
    let set_aside = &opened["turns"][0];
    assert_eq!(set_aside["state"], "stopped", "{set_aside}");
    assert_eq!(
        set_aside["answer"],
        "Stopped while waiting for your answer."
    );

    script.replace(vec![answers("Looped.")]);
    let next = send(address, &who, id, "loop it").await;
    assert_eq!(next.status, 202, "{}", next.body);
    let next = next.json()["id"].as_i64().unwrap();
    assert_ne!(next, turn, "a new turn, not the Gemini one resumed");
    let done = finished(address, &who, next).await;
    assert_eq!(done["turn"]["model"], "claude-sonnet-5-5");
    assert_eq!(done["turn"]["answer"], "Looped.");
}
