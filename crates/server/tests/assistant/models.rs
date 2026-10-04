//! The model a project's assistant runs on (#705): Gemini 3.8 Flash for a new
//! project, changeable at any time, each turn charged at its own model's
//! rates — and a conversation that comes along across a change.

use scorsese_server::assistant::Assistant;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::{Script, answers, calls, common, exchange, member, new_project, scripted, serve};

/// What one scripted call costs on Gemini 3.8 Flash: 1 000 × $0.75/M +
/// 500 × $3.75/M = 2 625 µ$, plus 10% and rounded up.
const GEMINI_CHARGED: i64 = 2_888;

async fn choose(
    address: std::net::SocketAddr,
    who: &str,
    id: i64,
    model: &str,
) -> common::Response {
    let path = format!("/api/projects/{id}/chat/model");
    let body = json!({ "model": model });
    common::request(address, "PUT", &path, &[who], Some(&body)).await
}

#[sqlx::test]
async fn a_new_project_runs_on_gemini_and_is_charged_at_its_rates(pool: PgPool) {
    let script = Script::new(vec![answers("Nothing yet.")]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = new_project(&pool, ana, json!([])).await;

    let chat = format!("/api/projects/{id}/chat");
    let before = common::request(address, "GET", &chat, &[&who], None)
        .await
        .json();
    assert_eq!(before["model"], "gemini-3.8-flash", "{before}");
    let ids: Vec<&str> = before["models"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        [
            "gemini-3.8-flash",
            "gemini-3.5-flash-lite",
            "claude-opus-5-5",
            "claude-sonnet-5-5"
        ]
    );
    assert_eq!(before["models"][0]["cache_seconds"], 3_600);
    assert_eq!(before["models"][0]["unavailable"], Value::Null);

    let detail = exchange(address, &who, id, "what is in it?").await;
    assert_eq!(detail["turn"]["model"], "gemini-3.8-flash");
    assert_eq!(detail["turn"]["charged_micros"], GEMINI_CHARGED, "{detail}");
    let first = script.parsed(0);
    assert_eq!(first[0]["role"], "user");
    assert_eq!(first[0]["parts"][0]["text"], "what is in it?");
    assert!(
        first[1]["parts"][0]["text"]
            .as_str()
            .unwrap()
            .starts_with("[scorsese server]")
    );
}

#[sqlx::test]
async fn a_conversation_survives_a_change_of_model_both_ways(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = super::project(&pool, ana, json!([])).await;
    script.replace(vec![
        calls("icons", json!({})),
        answers("Two icons."),
        answers("Gemini here."),
        answers("Claude again."),
    ]);
    exchange(address, &who, id, "which icons?").await;

    let chose = choose(address, &who, id, "gemini-3.5-flash-lite").await;
    assert_eq!(chose.status, 200, "{}", chose.body);
    assert_eq!(chose.json()["label"], "Gemini 3.5 Flash Lite");
    let on_gemini = exchange(address, &who, id, "thanks").await;
    assert_eq!(on_gemini["turn"]["model"], "gemini-3.5-flash-lite");
    // The Claude turn, in Gemini's wire: the call and its result, by name.
    let sent = script.parsed(2);
    assert_eq!(sent[2]["parts"][0]["functionCall"]["name"], "icons");
    assert_eq!(sent[3]["parts"][0]["functionResponse"]["name"], "icons");
    assert_eq!(sent[4]["parts"][0]["text"], "Two icons.");

    choose(address, &who, id, "claude-opus-5-5").await;
    exchange(address, &who, id, "and now?").await;
    // Back on Claude: the first turn replays its own bytes, thinking and all.
    let (earlier, now) = (script.messages(1), script.messages(3));
    assert_eq!(&now[..earlier.len()], &earlier[..]);
    // Then the rest, the Gemini turn translated back into Claude's wire.
    assert!(now[earlier.len() + 2].contains("Gemini here."), "{now:?}");
    assert!(!now[earlier.len() + 2].contains("thinking"), "{now:?}");
}

#[sqlx::test]
async fn only_an_offered_model_on_ones_own_project_can_be_chosen(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let (_, bea) = member(&pool, "bea@example.com", 10).await;
    let id = new_project(&pool, ana, json!([])).await;
    let unknown = choose(address, &who, id, "gemini-3.8-flash-lite").await;
    assert_eq!(unknown.status, 400, "{}", unknown.body);
    assert!(
        unknown.body.contains("gemini-3.8-flash"),
        "{}",
        unknown.body
    );
    let theirs = choose(address, &bea, id, "claude-opus-5-5").await;
    assert_eq!(theirs.status, 404, "{}", theirs.body);
}

#[sqlx::test]
async fn a_model_that_cannot_answer_says_why_in_the_picker(pool: PgPool) {
    let (address, _) = serve(&pool, Assistant::default().unconfigured()).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = new_project(&pool, ana, json!([])).await;
    let chat = format!("/api/projects/{id}/chat");
    let seen = common::request(address, "GET", &chat, &[&who], None)
        .await
        .json();
    for model in seen["models"].as_array().unwrap() {
        let why = model["unavailable"].as_str().unwrap_or_default();
        assert!(why.contains("not configured"), "{model}");
    }
}
