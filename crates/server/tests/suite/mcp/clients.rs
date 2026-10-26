//! Which client an external connection is (#707): the `clientInfo` of an
//! MCP `initialize`, kept on the token and recorded on its tool calls.

use serde_json::json;
use sqlx::postgres::PgPool;

use super::{call, common, member, post};

/// A `tool_calls` row's client, token, client name and client version.
type Row = (String, Option<i64>, Option<String>, Option<String>);

#[sqlx::test]
async fn an_initializes_client_info_lands_on_that_tokens_tool_calls(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    // Answered or refused, a call is on record; before any handshake, with
    // no client named.
    call(address, &who, "icons", json!({})).await;
    let initialize = json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "protocolVersion": "2025-06-18", "capabilities": {},
                    "clientInfo": { "name": "gemini-cli-mcp-client", "version": "0.9.1" } }
    });
    assert_eq!(post(address, &who, &initialize).await.status, 200);
    call(address, &who, "icons", json!({})).await;

    let mut tx = scorsese_server::db::scoped(&pool, ana).await.unwrap();
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT client, api_token_id, client_name, client_version FROM tool_calls ORDER BY id",
    )
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    let token: i64 = sqlx::query_scalar("SELECT id FROM api_tokens")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    let external = |name: Option<&str>, version: Option<&str>| {
        let owned = |s: Option<&str>| s.map(str::to_owned);
        (
            "external".to_owned(),
            Some(token),
            owned(name),
            owned(version),
        )
    };
    assert_eq!(
        rows,
        vec![
            external(None, None),
            external(Some("gemini-cli-mcp-client"), Some("0.9.1")),
        ]
    );
}
