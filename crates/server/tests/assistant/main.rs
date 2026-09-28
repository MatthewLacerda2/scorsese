//! The assistant (`src/assistant`, `src/http/chat.rs`): turns driven the way
//! the web app drives them — over HTTP — against a scripted Claude that
//! records every request it is sent. No test reaches Anthropic.

#[path = "../common/mod.rs"]
mod common;

mod money;
mod quotes;
mod resume;
mod script;
mod turns;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use scorsese_core::Project;
use scorsese_server::accounts::{tokens, users};
use scorsese_server::assistant::Assistant;
use scorsese_server::credits::ledger;
use scorsese_server::db::{self, UserId};
use scorsese_server::{http, projects};
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

pub(crate) use script::{Script, answers, calls};

/// A server whose assistant is `assistant`; its address and state.
async fn serve(pool: &PgPool, assistant: Assistant) -> (SocketAddr, http::AppState) {
    let (listener, address) = common::listener().await;
    scorsese_server::db::migrate(pool)
        .await
        .expect("the test setup holds");
    let members = scorsese_server::db::member_pool(pool)
        .await
        .expect("the test setup holds");
    let state = http::AppState::new(members, common::files("assistant")).with_assistant(assistant);
    tokio::spawn(http::serve(
        listener,
        http::router(state.clone()),
        std::future::pending(),
    ));
    (address, state)
}

/// A server answered by `script`, capped at a dollar a turn.
async fn scripted(pool: &PgPool, script: &Arc<Script>) -> (SocketAddr, http::AppState) {
    serve(
        pool,
        Assistant::new("claude-opus-5-5", 1_000_000).answered_by(script.clone()),
    )
    .await
}

/// A new account with `dollars` of credit, and its `Authorization:` line.
async fn member(pool: &PgPool, email: &str, dollars: i64) -> (UserId, String) {
    let user = users::create(pool, email, "password one")
        .await
        .expect("the test setup holds");
    if dollars > 0 {
        let mut tx = db::scoped(pool, user).await.expect("the test setup holds");
        ledger::top_up(&mut tx, dollars * 500, 50_000)
            .await
            .expect("the test setup holds");
        tx.commit().await.expect("the test setup holds");
    }
    let token = tokens::issue(pool, user, "laptop")
        .await
        .expect("the test setup holds")
        .token;
    (user, format!("Authorization: Bearer {token}"))
}

/// A project of `user`'s holding `assets`; its id.
async fn project(pool: &PgPool, user: UserId, assets: Value) -> i64 {
    let mut document = serde_json::to_value(Project::new("Intro", Default::default()))
        .expect("the test setup holds");
    document["assets"] = assets;
    let document = Project::from_json(&document.to_string()).expect("the test setup holds");
    projects::create(pool, user, &document)
        .await
        .expect("the test setup holds")
        .id
}

/// `prompt` sent about `project`: the response.
async fn send(address: SocketAddr, who: &str, project: i64, prompt: &str) -> common::Response {
    let path = format!("/api/projects/{project}/chat");
    let body = json!({ "prompt": prompt });
    common::request(address, "POST", &path, &[who], Some(&body)).await
}

/// Turn `turn`, with its tool log, once it has stopped running.
async fn finished(address: SocketAddr, who: &str, turn: i64) -> Value {
    let path = format!("/api/chat/turns/{turn}");
    for _ in 0..200 {
        let detail = common::request(address, "GET", &path, &[who], None)
            .await
            .json();
        if detail["turn"]["state"] != "running" {
            return detail;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("turn {turn} is still running");
}

/// Send `prompt` and wait for its turn to end: the turn's detail.
async fn exchange(address: SocketAddr, who: &str, project: i64, prompt: &str) -> Value {
    let sent = send(address, who, project, prompt).await;
    assert_eq!(sent.status, 202, "{}", sent.body);
    finished(
        address,
        who,
        sent.json()["id"].as_i64().expect("the test setup holds"),
    )
    .await
}
