//! Getting a session started, and surviving what a client gets wrong.

use crate::{exchange, once};
use serde_json::{Value, json};

#[test]
fn initialize_answers_with_what_this_server_is() {
    let reply = once(
        &json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "protocolVersion": "2025-06-18", "capabilities": {} }
        })
        .to_string(),
    );
    assert_eq!(reply["id"], json!(1));
    assert_eq!(reply["result"]["protocolVersion"], json!("2025-06-18"));
    assert_eq!(reply["result"]["serverInfo"]["name"], json!("scorsese"));
    assert!(reply["result"]["capabilities"]["tools"].is_object());
}

/// A client asking for a revision this server does not know still gets a
/// session: the server names one it can speak and the client decides.
#[test]
fn an_unknown_protocol_revision_is_answered_with_one_we_speak() {
    let reply = once(
        &json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "protocolVersion": "1999-01-01" }
        })
        .to_string(),
    );
    let offered = reply["result"]["protocolVersion"]
        .as_str()
        .expect("a version");
    assert_ne!(offered, "1999-01-01");
    assert!(offered.starts_with("20"), "got {offered}");
}

/// Notifications are acted on by existing. Replying to one is a protocol
/// error, and a server that does it desynchronises every id after it.
#[test]
fn a_notification_gets_no_reply_at_all() {
    let replies = exchange(&[&json!({
        "jsonrpc": "2.0", "method": "notifications/initialized"
    })
    .to_string()]);
    assert!(replies.is_empty(), "got {replies:?}");
}

/// One line of nonsense must not end the conversation. A client that fumbles a
/// frame has not hung up, and a server that exits takes the session with it.
#[test]
fn a_malformed_line_is_answered_and_the_session_carries_on() {
    let replies = exchange(&[
        "{not json at all",
        &json!({ "jsonrpc": "2.0", "id": 2, "method": "ping" }).to_string(),
    ]);
    assert_eq!(replies.len(), 2);
    assert_eq!(replies[0]["error"]["code"], json!(-32700));
    assert_eq!(replies[0]["id"], Value::Null);
    assert_eq!(replies[1]["id"], json!(2), "the next request still works");
}

#[test]
fn an_unknown_method_is_refused_by_name() {
    let reply = once(&json!({ "jsonrpc": "2.0", "id": 3, "method": "sing" }).to_string());
    assert_eq!(reply["error"]["code"], json!(-32601));
    assert!(
        reply["error"]["message"]
            .as_str()
            .is_some_and(|text| text.contains("sing")),
        "got {reply}"
    );
}

#[test]
fn blank_lines_are_skipped_rather_than_answered() {
    let replies = exchange(&[
        "",
        "   ",
        &json!({ "jsonrpc": "2.0", "id": 4, "method": "ping" }).to_string(),
    ]);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0]["id"], json!(4));
}
