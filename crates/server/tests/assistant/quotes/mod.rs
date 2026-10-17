//! A paid tool's quote goes to the user, never to the model; only the user's
//! yes spends. A quote pauses its turn, and the answer resumes the same turn
//! as the quoting call's result (#1440).

mod answers;
mod instead;

use std::net::SocketAddr;
use std::sync::Arc;

use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::{Script, answers, calls, common, exchange, member, project, scripted};

/// One line ready to speak: a quote with a price.
fn narrated() -> Value {
    json!([{ "id": "vo", "kind": "generated_audio", "state": "sketch",
             "prompt": "Every city has a night editor.", "speech": { "voice_id": "voice-one" } }])
}

/// A turn in which the model asks `generate` for a quote, and pauses; the
/// turn's detail.
async fn quoted(pool: &PgPool, script: &Arc<Script>) -> (SocketAddr, String, i64, Value) {
    let (address, _) = scripted(pool, script).await;
    let (ana, who) = member(pool, "ana@example.com", 10).await;
    let id = project(pool, ana, narrated()).await;
    script.replace(vec![calls("generate", json!({ "project": id }))]);
    let detail = exchange(address, &who, id, "narrate it").await;
    (address, who, id, detail)
}

/// The user's answer to turn `turn`'s quote.
async fn answer(address: SocketAddr, who: &str, turn: &Value, yes: bool) -> common::Response {
    let path = format!("/api/chat/turns/{}/quote", turn["turn"]["id"]);
    let body = json!({ "confirm": yes });
    common::request(address, "POST", &path, &[who], Some(&body)).await
}

/// How many quotes are waiting to be spent, anyone's.
async fn quotes_held(pool: &PgPool) -> i64 {
    count(pool, "SELECT count(*) FROM quotes").await
}

/// How many turns there are, anyone's: an answer never starts one.
async fn turns(pool: &PgPool) -> i64 {
    count(pool, "SELECT count(*) FROM chat_turns").await
}

async fn count(pool: &PgPool, sql: &'static str) -> i64 {
    sqlx::query_scalar(sql)
        .fetch_one(pool)
        .await
        .expect("the rows can be counted")
}

/// Every request the model was sent, as text: none may hold a token, or the
/// old flow's line.
fn never_told_a_token(script: &Script) {
    for n in 0..script.requests().len() {
        let sent = script.messages(n).join("\n");
        assert!(
            !sent.contains("quote-"),
            "the model never holds a token: {sent}"
        );
        assert!(
            !sent.contains("Yes, go ahead."),
            "nobody typed that: {sent}"
        );
    }
}

#[sqlx::test]
async fn a_quote_is_held_for_the_user_and_pauses_its_turn(pool: PgPool) {
    let script = Script::new(vec![]);
    let (_, _, _, detail) = quoted(&pool, &script).await;
    assert_eq!(detail["turn"]["state"], "asking", "{detail}");
    let quote = &detail["turn"]["quote"];
    assert_eq!(quote["tool"], "generate", "{detail}");
    assert!(quote["micros"].as_i64().unwrap() > 0, "{quote}");
    assert!(detail["turn"]["quote_answer"].is_null());
    assert_eq!(quote["items"][0]["subject"], "vo", "{quote}");
    assert!(
        !quote.to_string().contains("quote-"),
        "the box needs no token"
    );
    assert_eq!(script.requests().len(), 1, "the model waits, uncalled");
    assert_eq!(quotes_held(&pool).await, 1);
}

#[sqlx::test]
async fn a_model_naming_confirm_is_refused_and_nothing_is_spent(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, narrated()).await;
    let forged = json!({ "project": id, "confirm": "quote-0123456789abcdef01234567" });
    script.replace(vec![calls("generate", forged), answers("I could not.")]);
    let detail = exchange(address, &who, id, "just buy it").await;
    assert_eq!(detail["turn"]["state"], "answered", "nothing to pause on");
    let tools = detail["tools"].as_array().unwrap();
    assert_eq!(tools[0]["outcome"], "refused", "{detail}");
    let result = &script.parsed(1)[3]["content"][0];
    assert_eq!(result["is_error"], true);
    assert_eq!(count(&pool, "SELECT count(*) FROM jobs").await, 0);
}
