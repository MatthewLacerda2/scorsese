//! The assistant shows stock candidates and the user picks (#901): the turn
//! pauses without charging, and the pick — one or more (`picks`), none, or
//! words (`words`) — resumes it, what was picked imported first; a picker
//! naming results no search of the turn showed is refused and the turn goes
//! on (`refusals`). A Lottie is a candidate too, and a pick of one lands
//! under `pages/` (`lotties`).
//!
//! No test reaches Pixabay. The server's tools search and import from
//! [`Fake`], handed to it as its one stock library (#906): the scripted model
//! calls `stock_search` for real, its results land in the user's stock cache
//! and its reply in the turn's record, exactly as a live turn keeps them, and
//! a pick is imported from the same fake — as is the model's own
//! `stock_import` (`imports`).

mod fake;
mod imports;
mod lotties;
mod picks;
mod refusals;
mod words;

use std::net::SocketAddr;
use std::sync::Arc;

use scorsese_server::assistant::Assistant;
use scorsese_server::http::AppState;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::{Script, answers, calls, common, finished, member, project, send};
use fake::Fake;

/// A server answered by `script`, its stock tools and picks answering from
/// `fake`.
async fn stocked(pool: &PgPool, script: &Arc<Script>, fake: &Arc<Fake>) -> (SocketAddr, AppState) {
    let assistant = Assistant::new(1_000_000).answered_by(script.clone());
    super::serving(pool, assistant, Some(fake.clone())).await
}

/// A picker of images `ids`.
fn picker(ids: &[u64]) -> Value {
    let candidates: Vec<Value> = ids
        .iter()
        .map(|id| json!({ "kind": "image", "id": id }))
        .collect();
    json!({ "question": "Which sunrise do you like?", "candidates": candidates })
}

/// The answer `body` to turn `turn`'s question.
async fn answer(address: SocketAddr, who: &str, turn: i64, body: &Value) -> common::Response {
    let path = format!("/api/chat/turns/{turn}/answer");
    common::request(address, "POST", &path, &[who], Some(body)).await
}

/// A turn paused on a picker its own search showed.
struct Picking {
    script: Arc<Script>,
    fake: Arc<Fake>,
    address: SocketAddr,
    who: String,
    project: i64,
    turn: i64,
    /// The turn as it waits.
    paused: Value,
}

/// A turn that searched the fake for a sunrise, then showed a picker of
/// images `ids` and paused; once answered, the model says `done`.
async fn picking(pool: &PgPool, ids: &[u64], done: &str) -> Picking {
    let (script, fake) = (Script::new(Vec::new()), Fake::new());
    let (address, _) = stocked(pool, &script, &fake).await;
    let (ana, who) = member(pool, "ana@example.com", 10).await;
    let id = project(pool, ana, json!([])).await;
    let search = json!({ "project": id, "query": "sunrise", "kind": "image" });
    script.replace(vec![
        calls("stock_search", search),
        calls("pick_stock", picker(ids)),
        answers(done),
    ]);
    let sent = send(address, &who, id, "open on a sunrise").await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    let turn = sent.json()["id"].as_i64().expect("the turn has an id");
    let paused = finished(address, &who, turn).await;
    Picking {
        script,
        fake,
        address,
        who,
        project: id,
        turn,
        paused,
    }
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
