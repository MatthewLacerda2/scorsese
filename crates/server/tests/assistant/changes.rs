//! A quote's box shows what each item would send, and a change asked for in
//! it spends nothing: it starts the next turn with the user's words, framed
//! by the server as about the quoted items — on every model (#709).

use std::net::SocketAddr;

use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::{Script, answers, calls, common, exchange, finished, member, on_model, project};
use super::{scripted, send};

/// A shot and a line ready to send: a quote with a description for each.
fn briefed() -> Value {
    json!([
        { "id": "hero", "kind": "generated_video", "state": "sketch",
          "prompt": "A superhero in a red cape lands on a rooftop at dusk." },
        { "id": "vo", "kind": "generated_audio", "state": "sketch",
          "prompt": "Every city has a night editor.", "speech": { "voice_id": "voice-one" } }
    ])
}

/// The answer `body` to turn `turn`'s quote.
async fn answer(address: SocketAddr, who: &str, turn: &Value, body: Value) -> common::Response {
    let path = format!("/api/chat/turns/{}/quote", turn["turn"]["id"]);
    common::request(address, "POST", &path, &[who], Some(&body)).await
}

/// How many jobs anyone has queued: what spending would leave.
async fn jobs(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM jobs")
        .fetch_one(pool)
        .await
        .expect("the jobs can be counted")
}

/// A quote, then a change asked for, on a project on `model`; every request
/// the model was sent, as text.
async fn change_on(pool: &PgPool, model: &str) -> Vec<String> {
    let script = Script::new(vec![]);
    let (address, _) = scripted(pool, &script).await;
    let (ana, who) = member(pool, "ana@example.com", 10).await;
    let id = on_model(pool, ana, briefed(), model).await;
    script.replace(vec![
        calls("generate", json!({ "project": id })),
        answers("The shot and the line cost about a dollar; answer in the box."),
    ]);
    let detail = exchange(address, &who, id, "make it").await;
    let items = &detail["turn"]["quote"]["items"];
    assert_eq!(items[0]["subject"], "hero", "{detail}");
    assert_eq!(items[0]["brief"], "prompt");
    assert!(
        items[0]["description"]
            .as_str()
            .expect("the exchange went as scripted")
            .contains("red cape")
    );
    assert_eq!(items[1]["brief"], "line", "{items}");
    assert!(
        items[1]["says"]
            .as_str()
            .expect("the exchange went as scripted")
            .contains("characters")
    );

    script.replace(vec![answers("Yellow it is; here is the new quote.")]);
    let change = json!({ "confirm": false, "change": "make the cape yellow instead" });
    let changed = answer(address, &who, &detail, change).await;
    assert_eq!(changed.status, 200, "{}", changed.body);
    let changed = changed.json();
    assert!(changed["spent"].is_null(), "{changed}");
    let next = changed["turn"]["id"]
        .as_i64()
        .expect("the exchange went as scripted");
    let next = finished(address, &who, next).await;
    assert_eq!(next["turn"]["prompt"], "make the cape yellow instead");
    let first = finished(
        address,
        &who,
        detail["turn"]["id"]
            .as_i64()
            .expect("the exchange went as scripted"),
    )
    .await;
    assert_eq!(first["turn"]["quote_answer"], "declined");
    assert_eq!(jobs(pool).await, 0, "a change spends nothing");
    let again = answer(address, &who, &detail, json!({ "confirm": true })).await;
    assert_eq!(again.status, 400, "the quote is gone: {}", again.body);
    (0..script.requests().len())
        .map(|n| script.messages(n).join("\n"))
        .collect()
}

/// What every model must be sent for a change: the user's words as theirs,
/// the server's framing naming the items, and never a token.
fn told_alike(sent: &[String]) {
    let last = sent.last().expect("the exchange went as scripted");
    assert!(last.contains("make the cape yellow instead"), "{last}");
    assert!(last.contains("asks for a change"), "{last}");
    assert!(last.contains("(hero, vo)"), "{last}");
    assert!(last.contains("asset_set"), "{last}");
    for request in sent {
        assert!(!request.contains("quote-"), "no token, ever: {request}");
    }
}

#[sqlx::test]
async fn a_change_on_claude_is_the_users_message_and_a_server_note(pool: PgPool) {
    let sent = change_on(&pool, "claude-opus-5-5").await;
    told_alike(&sent);
    let last = sent.last().unwrap();
    assert!(last.contains(r#""role":"system""#), "{last}");
}

#[sqlx::test]
async fn a_change_on_gemini_is_the_users_message_and_a_server_note(pool: PgPool) {
    let sent = change_on(&pool, "gemini-3.8-flash").await;
    told_alike(&sent);
    assert!(sent.last().unwrap().contains("[scorsese server]"));
}

#[sqlx::test]
async fn an_empty_change_or_a_yes_with_one_answers_nothing(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, briefed()).await;
    script.replace(vec![
        calls("generate", json!({ "project": id })),
        answers("Quoted."),
    ]);
    let detail = exchange(address, &who, id, "make it").await;
    let blank = json!({ "confirm": false, "change": "  " });
    assert_eq!(answer(address, &who, &detail, blank).await.status, 400);
    let both = json!({ "confirm": true, "change": "yellow" });
    assert_eq!(answer(address, &who, &detail, both).await.status, 400);
    assert_eq!(jobs(&pool).await, 0);
    // Still waiting: a plain message withdraws it, as before.
    script.replace(vec![answers("Understood.")]);
    assert_eq!(send(address, &who, id, "never mind").await.status, 202);
}

#[sqlx::test]
async fn a_voice_designs_box_shows_the_voice_it_describes(pool: PgPool) {
    let script = Script::new(vec![]);
    let (address, _) = scripted(&pool, &script).await;
    let (ana, who) = member(&pool, "ana@example.com", 10).await;
    let id = project(&pool, ana, briefed()).await;
    let voice = "A warm, unhurried radio voice in her fifties, for late-night stories.";
    let text = "Every city has a night editor, and every night editor has a story about the \
                one that got away. This is that story, told the way she tells it.";
    let design = json!({ "project": id, "prompt": voice, "text": text, "seed": 7 });
    script.replace(vec![calls("voice_design", design), answers("Quoted.")]);
    let detail = exchange(address, &who, id, "design a narrator").await;
    let item = &detail["turn"]["quote"]["items"][0];
    assert_eq!(item["subject"], "design", "{detail}");
    assert_eq!(item["brief"], "voice");
    assert_eq!(item["description"], voice);
}
