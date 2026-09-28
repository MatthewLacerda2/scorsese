//! The HTTP half: who may post, what comes back, and how often.

use scorsese_server::accounts::sessions;
use scorsese_server::http::mcp::Limits;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{call, common, member, post};

#[sqlx::test]
async fn the_handshake_is_the_stdio_servers_own(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    let initialize = json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "protocolVersion": "2025-06-18", "capabilities": {} }
    });
    let reply = post(address, &ana, &initialize).await;
    assert_eq!(reply.status, 200, "{}", reply.body);
    assert!(
        reply
            .header("content-type")
            .unwrap()
            .starts_with("application/json")
    );
    let reply = reply.json();
    assert_eq!(reply["id"], json!(1));
    assert_eq!(reply["result"]["protocolVersion"], json!("2025-06-18"));
    assert_eq!(reply["result"]["serverInfo"]["name"], json!("scorsese"));

    let note = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
    let accepted = post(address, &ana, &note).await;
    assert_eq!(accepted.status, 202);
    assert!(accepted.body.is_empty());

    let batch = json!([
        { "jsonrpc": "2.0", "id": 2, "method": "ping" },
        { "jsonrpc": "2.0", "method": "notifications/initialized" },
        { "jsonrpc": "2.0", "id": 3, "method": "no/such" }
    ]);
    let replies = post(address, &ana, &batch).await.json();
    assert_eq!(replies.as_array().map(Vec::len), Some(2), "{replies}");
    assert_eq!(replies[1]["error"]["code"], json!(-32601));
}

/// A token, and only a token: no credential is `401`, a browser's session
/// `403`, and a token that is not one `401`.
#[sqlx::test]
async fn only_an_api_token_gets_in(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, _) = member(&pool, "ana@example.com").await;
    let ping = json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" });
    assert_eq!(post(address, "X-Nothing: at-all", &ping).await.status, 401);
    let bogus = "Authorization: Bearer scor_not-a-real-one";
    assert_eq!(post(address, bogus, &ping).await.status, 401);
    let cookie = sessions::open(&pool, ana).await.unwrap();
    let browser = format!("Cookie: scorsese_session={cookie}");
    assert_eq!(post(address, &browser, &ping).await.status, 403);
}

#[sqlx::test]
async fn no_stream_no_session_and_no_revision_it_does_not_speak(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (_, ana) = member(&pool, "ana@example.com").await;
    for method in ["GET", "DELETE"] {
        let refused = common::request(address, method, "/api/mcp", &[&ana], None).await;
        assert_eq!(refused.status, 405, "{method}");
        assert_eq!(refused.header("allow"), Some("POST"));
    }
    let ping = json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" });
    let old = common::request(
        address,
        "POST",
        "/api/mcp",
        &[&ana, "MCP-Protocol-Version: 1999-01-01"],
        Some(&ping),
    )
    .await;
    assert_eq!(old.status, 400, "{}", old.body);

    let garbled = common::send(address, "POST", "/api/mcp", &[&ana], b"{not json").await;
    assert_eq!(garbled.status, 400);
    assert_eq!(garbled.json()["error"]["code"], json!(-32700));
}

/// A client in a loop is told to wait; another user is not held up by it.
#[sqlx::test]
async fn a_runaway_client_is_told_to_wait(pool: PgPool) {
    let (_, mut state) = common::serve_with(pool.clone(), common::files("mcp")).await;
    // The server under test was made before this limit; a second router over
    // the same state, with a limit of two, is the one exercised.
    state.limits = Limits::new(2);
    let (listener, address_two) = common::listener().await;
    let router = scorsese_server::http::router(state);
    tokio::spawn(scorsese_server::http::serve(
        listener,
        router,
        std::future::pending(),
    ));
    let (_, ana) = member(&pool, "ana@example.com").await;
    let (_, bob) = member(&pool, "bob@example.com").await;
    for _ in 0..2 {
        assert!(!call(address_two, &ana, "project_list", json!({})).await.1);
    }
    let body = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": { "name": "project_list", "arguments": {} }
    });
    let refused = post(address_two, &ana, &body).await;
    assert_eq!(refused.status, 429, "{}", refused.body);
    assert!(refused.header("retry-after").is_some());
    let ping = json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" });
    assert_eq!(post(address_two, &ana, &ping).await.status, 200);
    assert!(!call(address_two, &bob, "project_list", json!({})).await.1);
}
