//! The assistant shows stock candidates and the user picks (#901): the turn
//! pauses without charging, and the pick — one or more, none, or words —
//! resumes it, what was picked imported first (`picks`); a picker naming
//! results no search of the turn showed is refused and the turn goes on
//! (`refusals`).
//!
//! No test reaches Pixabay. What a pick imports comes from [`Fake`], handed
//! to the assistant as its stock library. What a search *showed* cannot come
//! from a real `stock_search` here — that needs a key and the network — so
//! [`shown`] writes one into the turn the way the loop keeps it: a search of
//! the fake's into the user's own stock cache, and that search's reply words
//! in the turn's record.

mod fake;
mod picks;
mod refusals;

use std::net::SocketAddr;
use std::sync::Arc;

use scorsese_providers::chat::{Message, Part, ResultPart};
use scorsese_providers::stock::{self, Medium, Query};
use scorsese_server::assistant::Assistant;
use scorsese_server::db::UserId;
use scorsese_server::http::AppState;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::{Script, answers, calls, common, finished, member, project, send};
use fake::Fake;

/// A server answered by `script`, importing picks from `fake`.
async fn stocked(pool: &PgPool, script: &Arc<Script>, fake: &Arc<Fake>) -> (SocketAddr, AppState) {
    let assistant = Assistant::new(1_000_000)
        .answered_by(script.clone())
        .stocked_from(fake.clone());
    super::serve(pool, assistant).await
}

/// A picker of images `ids`.
fn picker(ids: &[u64]) -> Value {
    let candidates: Vec<Value> = ids
        .iter()
        .map(|id| json!({ "kind": "image", "id": id }))
        .collect();
    json!({ "question": "Which sunrise do you like?", "candidates": candidates })
}

/// Make turn `turn` — paused on a question — one that searched `fake` for a
/// sunrise and was shown images 1 to 3: the search kept in `user`'s stock
/// cache, and its reply's words in the turn's record before the question.
async fn shown(pool: &PgPool, state: &AppState, (user, turn): (UserId, i64), fake: &Fake) {
    let query = Query {
        medium: Medium::Image,
        words: "sunrise".into(),
        style: None,
        orientation: None,
        min_seconds: None,
        safe: true,
    };
    let cache = state.library.storage().stock(user);
    let found = stock::search(&cache, fake, &query, 1).expect("the fake answers");
    let listing: Vec<String> = found
        .candidates
        .iter()
        .enumerate()
        .map(|(index, one)| format!("{}. {}", index + 1, one.says()))
        .collect();
    let kept: String = sqlx::query_scalar("SELECT record FROM chat_turns WHERE id = $1")
        .bind(turn)
        .fetch_one(pool)
        .await
        .expect("the turn is kept");
    let mut record: Vec<Message> = serde_json::from_str(&kept).expect("a record");
    let asked = record.pop().expect("the question");
    let call = calls(
        "stock_search",
        json!({ "query": "sunrise", "kind": "image" }),
    );
    let Some(Part::Call { id, name, .. }) = call.message.parts().first().cloned() else {
        unreachable!()
    };
    let text = listing.join("\n");
    let result = Part::Result {
        call: id,
        name,
        content: vec![ResultPart::Text { text }],
        is_error: false,
    };
    record.extend([
        call.message,
        Message::User {
            content: vec![result],
        },
        asked,
    ]);
    sqlx::query("UPDATE chat_turns SET record = $2 WHERE id = $1")
        .bind(turn)
        .bind(serde_json::to_string(&record).expect("a record"))
        .execute(pool)
        .await
        .expect("the turn is kept");
}

/// The answer `body` to turn `turn`'s question.
async fn answer(address: SocketAddr, who: &str, turn: i64, body: &Value) -> common::Response {
    let path = format!("/api/chat/turns/{turn}/answer");
    common::request(address, "POST", &path, &[who], Some(body)).await
}

/// A turn paused on a picker of images 1 and 3 that its search showed: the
/// server, who, the project, the turn, and the turn as it waits.
async fn picking(
    pool: &PgPool,
    script: &Arc<Script>,
    fake: &Arc<Fake>,
) -> (SocketAddr, String, i64, i64, Value) {
    let (address, state) = stocked(pool, script, fake).await;
    let (ana, who) = member(pool, "ana@example.com", 10).await;
    let id = project(pool, ana, json!([])).await;
    let sent = send(address, &who, id, "open on a sunrise").await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    let turn = sent.json()["id"].as_i64().expect("the turn has an id");
    let first = finished(address, &who, turn).await;
    assert_eq!(first["turn"]["state"], "asking", "{first}");
    shown(pool, &state, (ana, turn), fake).await;
    let resumed = answer(address, &who, turn, &json!({ "answer": "a photo" })).await;
    assert_eq!(resumed.status, 202, "{}", resumed.body);
    let paused = finished(address, &who, turn).await;
    (address, who, id, turn, paused)
}

/// The script every picking turn follows: a question first — which the
/// search is written in behind — then the picker, then `done`.
fn script(ids: &[u64], done: &str) -> Arc<Script> {
    let question = json!({ "question": "A photo or footage?", "options": ["photo", "footage"] });
    Script::new(vec![
        calls("ask_user", question),
        calls("pick_stock", picker(ids)),
        answers(done),
    ])
}

/// What the model was last told, in words.
fn told(script: &Script, request: usize) -> String {
    let sent = script.parsed(request);
    let last = sent.last().expect("a message");
    last["content"][0]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}
