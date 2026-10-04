//! A paid tool's quote goes to the user, never to the model; only the user's
//! yes spends.

use std::net::SocketAddr;
use std::sync::Arc;

use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::{Script, answers, calls, common, exchange, finished, member, project, scripted};

/// One line ready to speak: a quote with a price.
fn narrated() -> Value {
    json!([{ "id": "vo", "kind": "generated_audio", "state": "sketch",
             "prompt": "Every city has a night editor.", "speech": { "voice_id": "voice-one" } }])
}

/// A turn in which the model asks `generate` for a quote; the turn's detail.
async fn quoted(pool: &PgPool, script: &Arc<Script>) -> (SocketAddr, String, i64, Value) {
    let (address, _) = scripted(pool, script).await;
    let (ana, who) = member(pool, "ana@example.com", 10).await;
    let id = project(pool, ana, narrated()).await;
    script.replace(vec![
        calls("generate", json!({ "project": id })),
        answers("That line costs about two cents; say yes in the box to go ahead."),
    ]);
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
    sqlx::query_scalar("SELECT count(*) FROM quotes")
        .fetch_one(pool)
        .await
        .expect("the quotes can be counted")
}

#[sqlx::test]
async fn the_quote_is_held_for_the_user_and_its_token_never_reaches_the_model(pool: PgPool) {
    let script = Script::new(vec![]);
    let (_, _, _, detail) = quoted(&pool, &script).await;
    let quote = &detail["turn"]["quote"];
    assert_eq!(quote["tool"], "generate", "{detail}");
    assert!(quote["micros"].as_i64().unwrap() > 0, "{quote}");
    assert!(detail["turn"]["quote_answer"].is_null());
    assert_eq!(quote["items"][0]["subject"], "vo", "{quote}");
    let shown = quote.to_string();
    assert!(!shown.contains("quote-"), "the box needs no token: {shown}");
    let result = script.messages(1).last().unwrap().clone();
    assert!(result.contains("confirmation box"), "{result}");
    assert!(
        !result.contains("quote-"),
        "the model must never hold a token: {result}"
    );
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
    let tools = detail["tools"].as_array().unwrap();
    assert_eq!(tools[0]["outcome"], "refused", "{detail}");
    let result = &script.parsed(1)[3]["content"][0];
    assert_eq!(result["is_error"], true);
    let jobs: i64 = sqlx::query_scalar("SELECT count(*) FROM jobs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(jobs, 0);
}

#[sqlx::test]
async fn a_yes_spends_as_the_user_and_the_model_is_told_by_the_server(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, who, _, detail) = quoted(&pool, &script).await;
    script.replace(vec![answers("The line is being made.")]);
    let answered = answer(address, &who, &detail, true).await;
    assert_eq!(answered.status, 200, "{}", answered.body);
    let answered = answered.json();
    assert_eq!(answered["refused"], false, "{answered}");
    assert!(
        answered["spent"]
            .as_str()
            .unwrap()
            .contains("queued as job"),
        "{answered}"
    );
    let next = finished(address, &who, answered["turn"]["id"].as_i64().unwrap()).await;
    assert_eq!(next["turn"]["answer"], "The line is being made.");

    let first = detail["turn"]["id"].as_i64().unwrap();
    let log = finished(address, &who, first).await;
    let yes = log["tools"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(
        (yes["client"].clone(), yes["tool"].clone()),
        (json!("user"), json!("generate"))
    );
    assert_eq!(log["turn"]["quote_answer"], "confirmed");
    let told = script.parsed(2);
    let note = told.last().unwrap();
    assert_eq!(note["role"], "system", "{told:?}");
    assert!(
        note["content"]
            .as_str()
            .unwrap()
            .contains("The person confirmed the quote")
    );
    for n in 0..script.requests().len() {
        let sent = script.messages(n).join("\n");
        assert!(!sent.contains("quote-"), "a yes shows no token: {sent}");
    }
    let again = answer(address, &who, &detail, true).await;
    assert_eq!(
        again.status, 400,
        "a quote is answered once: {}",
        again.body
    );
}

#[sqlx::test]
async fn a_no_withdraws_the_quote_and_the_next_turn_is_told(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, who, id, detail) = quoted(&pool, &script).await;
    let declined = answer(address, &who, &detail, false).await;
    assert_eq!(declined.status, 200, "{}", declined.body);
    assert!(declined.json()["turn"].is_null());
    assert_eq!(quotes_held(&pool).await, 0);

    script.replace(vec![answers("Understood.")]);
    exchange(address, &who, id, "fine, leave it").await;
    let told = script.parsed(2);
    let note = told.last().unwrap();
    assert!(
        note["content"].as_str().unwrap().contains("declined"),
        "{told:?}"
    );
}
