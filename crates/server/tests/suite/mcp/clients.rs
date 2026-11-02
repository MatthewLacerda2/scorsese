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

#[sqlx::test]
async fn a_client_name_and_version_are_kept_to_two_hundred_characters(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let initialize = |name: &str, version: &str| {
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "protocolVersion": "2025-06-18", "capabilities": {},
                        "clientInfo": { "name": name, "version": version } }
        })
    };
    let kept = || async {
        let mut tx = scorsese_server::db::scoped(&pool, ana).await.unwrap();
        let row: (Option<String>, Option<String>) =
            sqlx::query_as("SELECT client_name, client_version FROM api_tokens")
                .fetch_one(&mut *tx)
                .await
                .unwrap();
        row
    };
    let at = "n".repeat(200);
    let handshake = initialize(&at, &"9".repeat(200));
    assert_eq!(post(address, &who, &handshake).await.status, 200);
    assert_eq!(kept().await, (Some(at.clone()), Some("9".repeat(200))));

    let past = "é".repeat(201);
    let handshake = initialize(&past, &"9".repeat(201));
    assert_eq!(post(address, &who, &handshake).await.status, 200);
    assert_eq!(
        kept().await,
        (Some("é".repeat(200)), Some("9".repeat(200))),
        "a longer name is cut, and the handshake still answered"
    );
}

#[sqlx::test]
async fn a_calls_arguments_are_kept_to_sixteen_kibibytes(pool: PgPool) {
    let address = common::serve(pool.clone()).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let most = 16 * 1024;
    // `{"pad":"…"}` is 10 bytes around the padding.
    let at = json!({ "pad": "x".repeat(most - 10) });
    assert_eq!(serde_json::to_string(&at).unwrap().len(), most);
    let past = json!({ "pad": "x".repeat(most - 9) });
    call(address, &who, "icons", at.clone()).await;
    call(address, &who, "icons", past).await;

    let mut tx = scorsese_server::db::scoped(&pool, ana).await.unwrap();
    let rows: Vec<serde_json::Value> =
        sqlx::query_scalar("SELECT arguments FROM tool_calls ORDER BY id")
            .fetch_all(&mut *tx)
            .await
            .unwrap();
    assert_eq!(rows[0], at, "at the bound, kept whole");
    let cut = serde_json::to_string(&rows[1]).unwrap();
    assert!(cut.len() <= most, "{} bytes kept", cut.len());
    assert_eq!(rows[1]["cut"], true, "the cut says so");
    assert_eq!(rows[1]["bytes"], most + 1, "and how long it was");
    assert!(
        rows[1]["kept"]
            .as_str()
            .unwrap()
            .starts_with("{\"pad\":\"xxx"),
        "and keeps its beginning"
    );
}
