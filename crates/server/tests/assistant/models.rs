//! The model a project's assistant runs on (#705): Claude Sonnet 5.5 for a new
//! project, changeable at any time, each turn charged at its own model's
//! rates — and a conversation that comes along across a change.

use scorsese_server::assistant::Assistant;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::script::CHARGED;
use super::{Script, answers, calls, common, exchange, member, new_project, scripted, serve};

/// What one scripted call is charged on Sonnet 5.5: 1 000 × $2/M + 500 ×
/// $10/M = 7 000 µ$, times 1.8 — half of Opus's [`CHARGED`].
const SONNET_CHARGED: i64 = 12_600;

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
async fn a_new_project_starts_on_sonnet_and_opus_charges_its_own_rates(pool: PgPool) {
    let script = Script::new(vec![answers("Nothing yet."), answers("Still nothing.")]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = new_project(&pool, ana, json!([])).await;

    let chat = format!("/api/projects/{id}/chat");
    let before = common::request(address, "GET", &chat, &[&who], None)
        .await
        .json();
    assert_eq!(before["model"], "claude-sonnet-5-5", "{before}");
    let offered: Vec<(&Value, &Value, &Value)> = before["models"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| (&m["id"], &m["vendor"], &m["cost"]))
        .collect();
    // The bar's length: Opus is the dearest, Sonnet half of it (#706).
    assert_eq!(
        offered,
        [
            (&json!("claude-sonnet-5-5"), &json!("anthropic"), &json!(50)),
            (&json!("claude-opus-5-5"), &json!("anthropic"), &json!(100)),
        ]
    );
    assert_eq!(before["models"][0]["cache_seconds"], 3_600);
    assert_eq!(before["models"][0]["unavailable"], Value::Null);

    let on_sonnet = exchange(address, &who, id, "what is in it?").await;
    assert_eq!(on_sonnet["turn"]["model"], "claude-sonnet-5-5");
    assert_eq!(on_sonnet["turn"]["charged_micros"], SONNET_CHARGED);
    let chose = choose(address, &who, id, "claude-opus-5-5").await;
    assert_eq!(chose.status, 200, "{}", chose.body);
    let on_opus = exchange(address, &who, id, "and now?").await;
    assert_eq!(on_opus["turn"]["model"], "claude-opus-5-5");
    assert_eq!(on_opus["turn"]["charged_micros"], CHARGED, "{on_opus}");
}

/// Sonnet and Opus share a wire but not each other's thinking: a switch
/// either way translates the other model's turns from their record, and a
/// model's own turns replay as they were sent.
#[sqlx::test]
async fn a_conversation_survives_a_change_of_model_both_ways(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = super::project(&pool, ana, json!([])).await;
    script.replace(vec![
        calls("icons", json!({})),
        answers("Two icons."),
        answers("Sonnet here."),
        answers("Opus again."),
    ]);
    exchange(address, &who, id, "which icons?").await;

    let chose = choose(address, &who, id, "claude-sonnet-5-5").await;
    assert_eq!(chose.status, 200, "{}", chose.body);
    assert_eq!(chose.json()["label"], "Claude Sonnet 5.5");
    let on_sonnet = exchange(address, &who, id, "thanks").await;
    assert_eq!(on_sonnet["turn"]["model"], "claude-sonnet-5-5");
    // The Opus turn, from its record: the call and its result, no thinking.
    let sent = script.messages(2);
    assert!(sent[2].contains("\"tool_use\"") && sent[3].contains("\"tool_result\""));
    assert!(sent[4].contains("Two icons."), "{sent:?}");
    assert!(sent.iter().all(|m| !m.contains("thinking")), "{sent:?}");

    choose(address, &who, id, "claude-opus-5-5").await;
    exchange(address, &who, id, "and now?").await;
    // Back on Opus: the first turn replays its own bytes, thinking and all.
    let (earlier, now) = (script.messages(1), script.messages(3));
    assert_eq!(&now[..earlier.len()], &earlier[..]);
    assert!(now[earlier.len()].contains("thinking"), "{now:?}");
    // Then the Sonnet turn, translated, without Sonnet's thinking.
    assert!(now[earlier.len() + 2].contains("Sonnet here."), "{now:?}");
    assert!(!now[earlier.len() + 2].contains("thinking"), "{now:?}");
}

#[sqlx::test]
async fn only_an_offered_model_on_ones_own_project_can_be_chosen(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let (_, bea) = member(&pool, "bea@example.com", 10).await;
    let id = new_project(&pool, ana, json!([])).await;
    let cut = choose(address, &who, id, "gemini-3.8-flash").await;
    assert_eq!(cut.status, 400, "{}", cut.body);
    assert!(cut.body.contains("claude-sonnet-5-5"), "{}", cut.body);
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
