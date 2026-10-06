//! Web MCP (`src/tools`, `src/http/mcp.rs`): scorsese's tools over HTTP, for
//! the user an API token names — driven the way a client drives it, JSON-RPC
//! posted to `/api/mcp`.

#[path = "../common/mod.rs"]
mod common;

mod clients;
mod described;
mod designing;
mod drawing;
mod editing;
mod importing;
mod pages;
mod paying;
mod recipes;
mod settling;
mod shooting;
mod shortening;
mod stopping;
mod studio;
mod transcribing;
mod transport;
mod vendors;

use std::net::SocketAddr;

use scorsese_core::Project;
use scorsese_server::accounts::{tokens, users};
use scorsese_server::db::UserId;
use scorsese_server::projects;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;

/// A new account, and the `Authorization:` line of an API token for it.
async fn member(pool: &PgPool, email: &str) -> (UserId, String) {
    let user = users::create(pool, email, "password one")
        .await
        .expect("the account is created");
    let issued = tokens::issue(pool, user, "laptop")
        .await
        .expect("a token is issued");
    (user, format!("Authorization: Bearer {}", issued.token))
}

/// `body` posted to `/api/mcp` as `who`.
async fn post(address: SocketAddr, who: &str, body: &Value) -> common::Response {
    common::request(address, "POST", "/api/mcp", &[who], Some(body)).await
}

/// A `tools/call` of `tool` with `arguments`: the words it answered with, and
/// whether it refused.
async fn call(address: SocketAddr, who: &str, tool: &str, arguments: Value) -> (String, bool) {
    let body = json!({
        "jsonrpc": "2.0", "id": 7, "method": "tools/call",
        "params": { "name": tool, "arguments": arguments }
    });
    let response = post(address, who, &body).await;
    assert_eq!(response.status, 200, "{}", response.body);
    let reply = response.json();
    let text = reply["result"]["content"]
        .as_array()
        .unwrap_or_else(|| panic!("no content in {reply}"))
        .iter()
        .filter_map(|block| block["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    (text, reply["result"]["isError"] == json!(true))
}

/// `document`, stored as one of `user`'s projects; its id.
async fn stored(pool: &PgPool, user: UserId, document: Value) -> i64 {
    let mut project = serde_json::to_value(Project::new("film", Default::default()))
        .expect("a new project serialises");
    for (key, value) in document.as_object().expect("an object").clone() {
        project[key] = value;
    }
    let project = Project::from_json(&project.to_string()).expect("the document is a project");
    projects::create(pool, user, &project)
        .await
        .expect("the project is stored")
        .id
}

/// `user`'s project `id` as it is stored now.
async fn document(pool: &PgPool, user: UserId, id: i64) -> projects::Stored {
    projects::open(pool, user, id)
        .await
        .expect("the project opens")
}
