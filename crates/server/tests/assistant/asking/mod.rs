//! The assistant asks a question mid-edit (#710): the turn pauses without
//! charging, and the answer — from the card or a typed message — resumes the
//! same turn as the call's result, on Claude and on Gemini alike (`resumes`);
//! a question asked wrongly, or set aside, ends nothing it should not
//! (`ends`).

mod ends;
mod resumes;

use std::net::SocketAddr;
use std::time::Duration;

use serde_json::{Value, json};
use sqlx::postgres::PgPool;

use super::script::CHARGED;
use super::{Script, answers, calls, common, finished, member, on_model, project, scripted, send};

/// What the scripted model asks.
fn question() -> Value {
    json!({ "question": "The music ends early: loop it or fade it?",
            "options": ["loop it", "fade it"] })
}

/// `prompt` sent, and the turn once it stops running.
async fn asked(address: SocketAddr, who: &str, id: i64, prompt: &str) -> Value {
    let sent = send(address, who, id, prompt).await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    let turn = sent.json()["id"].as_i64().expect("the turn has an id");
    finished(address, who, turn).await
}

/// `answer` to turn `turn`'s question.
async fn answer(address: SocketAddr, who: &str, turn: i64, answer: &str) -> common::Response {
    let path = format!("/api/chat/turns/{turn}/answer");
    let body = json!({ "answer": answer });
    common::request(address, "POST", &path, &[who], Some(&body)).await
}
