//! A turn cut off — by a restart, or by a refusal — leaves a conversation the
//! next turn can still continue.

use scorsese_providers::chat::{Message, Stop};
use scorsese_server::assistant;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{Script, answers, exchange, member, project, scripted};

#[sqlx::test]
async fn a_turn_the_server_died_in_is_interrupted_and_its_calls_answered_next(pool: PgPool) {
    let script = Script::new(vec![answers("Carrying on.")]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let cut = r#"[{"role":"user","content":[{"type":"text","text":"look"}]},{"role":"assistant","content":[{"type":"tool_use","id":"toolu_lost","name":"icons","input":{}}]}]"#;
    let session: i64 = sqlx::query_scalar(
        "INSERT INTO chat_sessions (user_id, project_id) VALUES ($1, $2) RETURNING id",
    )
    .bind(ana.get())
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO chat_turns (user_id, session_id, prompt, messages, model, effort)
         VALUES ($1, $2, 'look', $3, 'claude-opus-5-5', 'high')",
    )
    .bind(ana.get())
    .bind(session)
    .bind(cut)
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(assistant::recover(&pool).await.unwrap(), 1);

    let detail = exchange(address, &who, id, "go on").await;
    assert_eq!(detail["turn"]["answer"], "Carrying on.");
    let sent = script.messages(0);
    assert_eq!(
        &sent[..2].join(","),
        &cut[1..cut.len() - 1],
        "replayed as it was kept"
    );
    let opening = script.parsed(0)[2].clone();
    assert_eq!(
        opening["content"][0]["tool_use_id"], "toolu_lost",
        "{opening}"
    );
    assert_eq!(opening["content"][0]["is_error"], true);
    assert_eq!(opening["content"][1]["text"], "go on");
    let first = super::common::request(
        address,
        "GET",
        &format!("/api/projects/{id}/chat"),
        &[&who],
        None,
    )
    .await
    .json();
    assert_eq!(first["turns"][0]["state"], "interrupted", "{first}");
}

#[sqlx::test]
async fn a_refused_turn_is_closed_before_the_next_one_speaks(pool: PgPool) {
    let mut refusal = super::script::answers("");
    refusal.message = Message::Assistant { content: vec![] };
    refusal.stop = Stop::Refusal {
        category: Some("cyber".into()),
        explanation: Some("declined".into()),
    };
    let script = Script::new(vec![refusal, answers("Happy to help with that.")]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;

    let refused = exchange(address, &who, id, "something odd").await;
    assert_eq!(refused["turn"]["state"], "refused", "{refused}");
    assert!(
        refused["turn"]["answer"]
            .as_str()
            .unwrap()
            .contains("declined")
    );
    assert_eq!(
        refused["turn"]["calls"], 1,
        "a refusal is still a call, and charged"
    );

    exchange(address, &who, id, "make a title instead").await;
    let roles: Vec<String> = script
        .parsed(1)
        .iter()
        .map(|message| message["role"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(roles, ["user", "system", "assistant", "user"]);
}
