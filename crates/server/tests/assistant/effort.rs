//! How hard the model thinks is the user's, per message (#769), and kept by a
//! turn that resumes; and what it thinks never reaches the browser (#767).

use std::net::SocketAddr;
use std::time::Duration;

use futures_util::StreamExt;
use scorsese_providers::chat::Effort;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::script::THOUGHT;
use super::{Script, answers, calls, common, finished, member, project, scripted};

/// `prompt` sent about `project` at `effort`: the response.
async fn send_at(
    address: SocketAddr,
    who: &str,
    project: i64,
    prompt: &str,
    effort: &str,
) -> common::Response {
    let path = format!("/api/projects/{project}/chat");
    let body = json!({ "prompt": prompt, "effort": effort });
    common::request(address, "POST", &path, &[who], Some(&body)).await
}

/// The effort turn `turn` recorded.
async fn recorded(pool: &PgPool, turn: i64) -> String {
    sqlx::query_scalar("SELECT effort FROM chat_turns WHERE id = $1")
        .bind(turn)
        .fetch_one(pool)
        .await
        .expect("the turn is stored")
}

#[sqlx::test]
async fn a_message_is_answered_at_its_own_effort_and_high_without_one(pool: PgPool) {
    let script = Script::new(vec![answers("Bigger."), answers("Built.")]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;

    let quick = send_at(address, &who, id, "make the title bigger", "low").await;
    assert_eq!(quick.status, 202, "{}", quick.body);
    let turn = quick.json()["id"].as_i64().unwrap();
    finished(address, &who, turn).await;
    assert_eq!(script.requests()[0].effort, Effort::Low);
    assert_eq!(recorded(&pool, turn).await, "low");

    let detail = super::exchange(address, &who, id, "now build the whole cut").await;
    assert_eq!(script.requests()[1].effort, Effort::High, "absent is high");
    let turn = detail["turn"]["id"].as_i64().unwrap();
    assert_eq!(recorded(&pool, turn).await, "high");

    let refused = send_at(address, &who, id, "think harder", "max").await;
    assert_eq!(refused.status, 400, "{}", refused.body);
    assert_eq!(script.requests().len(), 2, "nothing was sent for it");
}

#[sqlx::test]
async fn an_answered_question_resumes_at_the_effort_the_turn_began_with(pool: PgPool) {
    let question = json!({ "question": "Loop it or fade it?", "options": ["loop", "fade"] });
    let script = Script::new(vec![calls("ask_user", question), answers("Faded.")]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let sent = send_at(address, &who, id, "add the music", "medium").await;
    let turn = sent.json()["id"].as_i64().unwrap();
    assert_eq!(
        finished(address, &who, turn).await["turn"]["state"],
        "asking"
    );

    let path = format!("/api/chat/turns/{turn}/answer");
    let body = json!({ "answer": "fade" });
    let resumed = common::request(address, "POST", &path, &[&who], Some(&body)).await;
    assert_eq!(resumed.status, 202, "{}", resumed.body);
    finished(address, &who, turn).await;
    let efforts: Vec<Effort> = script.requests().iter().map(|r| r.effort).collect();
    assert_eq!(efforts, [Effort::Medium, Effort::Medium]);
}

#[sqlx::test]
async fn a_quote_answered_yes_carries_on_at_the_quoted_turns_effort(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let line = json!([{ "id": "vo", "kind": "generated_audio", "state": "sketch",
        "prompt": "Every city has a night editor.", "speech": { "voice_id": "voice-one" } }]);
    let id = project(&pool, ana, line).await;
    script.replace(vec![
        calls("generate", json!({ "project": id })),
        answers("About two cents; say yes to go ahead."),
        answers("Done."),
    ]);
    let sent = send_at(address, &who, id, "narrate it", "low").await;
    let turn = sent.json()["id"].as_i64().unwrap();
    finished(address, &who, turn).await;

    let path = format!("/api/chat/turns/{turn}/quote");
    let yes = json!({ "confirm": true });
    let answered = common::request(address, "POST", &path, &[&who], Some(&yes)).await;
    assert_eq!(answered.status, 200, "{}", answered.body);
    let next = answered.json()["turn"]["id"]
        .as_i64()
        .expect("a turn carries on");
    finished(address, &who, next).await;
    assert_eq!(script.requests().last().unwrap().effort, Effort::Low);
    assert_eq!(recorded(&pool, next).await, "low");
}

#[sqlx::test]
async fn the_models_thinking_never_reaches_the_event_stream(pool: PgPool) {
    let script = Script::new(vec![answers("Trimmed.")]);
    let (address, state) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let mut events = Box::pin(state.events.subscribe(ana));
    super::exchange(address, &who, id, "trim the intro").await;

    let mut heard = Vec::new();
    let quiet = Duration::from_millis(200);
    while let Ok(Some(event)) = tokio::time::timeout(quiet, events.next()).await {
        heard.push(serde_json::to_string(&event).unwrap());
    }
    assert!(heard.iter().any(|e| e.contains("Trimmed.")), "{heard:?}");
    assert!(!heard.iter().any(|e| e.contains(THOUGHT)), "{heard:?}");
}
