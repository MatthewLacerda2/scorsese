//! How hard the model thinks is the project's (#769, #1443), changed without
//! losing the cache and kept by a turn that resumes; and what it thinks never
//! reaches the browser (#767).

use std::net::SocketAddr;
use std::time::Duration;

use futures_util::StreamExt;
use scorsese_providers::chat::Effort;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::script::THOUGHT;
use super::{Script, answers, calls, common, finished, member, project, scripted};

/// `project`'s effort set to `effort` by `who`: the response.
async fn choose(address: SocketAddr, who: &str, project: i64, effort: &str) -> common::Response {
    let path = format!("/api/projects/{project}/chat/effort");
    let body = json!({ "effort": effort });
    common::request(address, "PUT", &path, &[who], Some(&body)).await
}

/// `prompt` sent about `project` at `effort`, the project's level from then
/// on: the turn's id.
async fn send_at(address: SocketAddr, who: &str, project: i64, prompt: &str, effort: &str) -> i64 {
    let chosen = choose(address, who, project, effort).await;
    assert_eq!(chosen.status, 200, "{}", chosen.body);
    assert_eq!(chosen.json(), json!({ "effort": effort }));
    let path = format!("/api/projects/{project}/chat");
    let body = json!({ "prompt": prompt });
    let sent = common::request(address, "POST", &path, &[who], Some(&body)).await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    sent.json()["id"].as_i64().expect("a turn starts")
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
async fn a_project_runs_at_its_own_effort_and_a_change_keeps_the_cache(pool: PgPool) {
    let script = Script::new(vec![answers("Bigger."), answers("Built.")]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let path = format!("/api/projects/{id}/chat");
    let stored = common::request(address, "GET", &path, &[&who], None).await;
    assert_eq!(stored.json()["effort"], "high", "a new project's");

    let turn = send_at(address, &who, id, "make the title bigger", "low").await;
    finished(address, &who, turn).await;
    assert_eq!(script.requests()[0].effort(), Effort::Low);
    assert_eq!(recorded(&pool, turn).await, "low");

    let turn = send_at(address, &who, id, "now build the whole cut", "xhigh").await;
    finished(address, &who, turn).await;
    let requests = script.requests();
    assert_eq!(requests[1].effort(), Effort::Xhigh);
    assert_eq!(recorded(&pool, turn).await, "xhigh");
    let before: Vec<&str> = requests[0].messages.iter().map(|m| m.get()).collect();
    let after: Vec<&str> = requests[1].messages.iter().map(|m| m.get()).collect();
    assert_eq!(
        after[..before.len()],
        before,
        "the history the cache holds is untouched"
    );
    let stored = common::request(address, "GET", &path, &[&who], None).await;
    assert_eq!(stored.json()["effort"], "xhigh");

    let refused = choose(address, &who, id, "max").await;
    assert_eq!(refused.status, 400, "{}", refused.body);
    let stored = common::request(address, "GET", &path, &[&who], None).await;
    assert_eq!(stored.json()["effort"], "xhigh", "unchanged");
}

#[sqlx::test]
async fn an_answered_question_resumes_at_the_effort_the_turn_began_with(pool: PgPool) {
    let question = json!({ "question": "Loop it or fade it?", "options": ["loop", "fade"] });
    let script = Script::new(vec![calls("ask_user", question), answers("Faded.")]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, json!([])).await;
    let turn = send_at(address, &who, id, "add the music", "medium").await;
    assert_eq!(
        finished(address, &who, turn).await["turn"]["state"],
        "asking"
    );

    let path = format!("/api/chat/turns/{turn}/answer");
    let body = json!({ "answer": "fade" });
    let resumed = common::request(address, "POST", &path, &[&who], Some(&body)).await;
    assert_eq!(resumed.status, 202, "{}", resumed.body);
    finished(address, &who, turn).await;
    let efforts: Vec<Effort> = script.requests().iter().map(|r| r.effort()).collect();
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
    let turn = send_at(address, &who, id, "narrate it", "low").await;
    finished(address, &who, turn).await;
    // A change of the project's level waits for the next message.
    assert_eq!(choose(address, &who, id, "high").await.status, 200);

    let path = format!("/api/chat/turns/{turn}/quote");
    let yes = json!({ "confirm": true });
    let answered = common::request(address, "POST", &path, &[&who], Some(&yes)).await;
    assert_eq!(answered.status, 200, "{}", answered.body);
    let next = answered.json()["turn"]["id"]
        .as_i64()
        .expect("a turn carries on");
    finished(address, &who, next).await;
    assert_eq!(script.requests().last().unwrap().effort(), Effort::Low);
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
