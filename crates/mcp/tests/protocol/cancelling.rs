//! A client cancelling a call in flight (#647): the render stops, its file
//! goes, nothing answers the cancelled request, and the session carries on.
//!
//! The cancel is the very next line after the call, which is how a client's
//! "stop" looks on the wire, and the reader sees it while the render is still
//! probing or drawing — the fixture is twenty seconds long, far more than the
//! moment it takes to read one line. Where in the render it lands is the
//! render crate's test to pin down; this one is about the protocol.

use serde_json::json;

use super::fixture::project;
use crate::exchange;

#[test]
fn a_cancelled_render_is_not_answered_and_leaves_no_file() {
    let dir = project("render-cancelled");
    let out = dir.join("cut.mp4");
    let render = json!({
        "jsonrpc": "2.0", "id": 7, "method": "tools/call",
        "params": { "name": "render", "arguments": {
            "project": dir, "out": out, "resolution": "160x90"
        } }
    });
    let cancel = json!({
        "jsonrpc": "2.0", "method": "notifications/cancelled",
        "params": { "requestId": 7, "reason": "the user pressed stop" }
    });
    let ping = json!({ "jsonrpc": "2.0", "id": 8, "method": "ping" });

    let replies = exchange(&[&render.to_string(), &cancel.to_string(), &ping.to_string()]);

    let ids: Vec<_> = replies.iter().map(|reply| reply["id"].clone()).collect();
    assert_eq!(
        ids,
        vec![json!(8)],
        "only the ping is answered: {replies:?}"
    );
    assert!(!out.exists(), "the unfinished file was left at {out:?}");
    std::fs::remove_dir_all(dir).ok();
}

/// A cancel naming a request that is not running — finished, never sent, or a
/// string where the call used a number — changes nothing.
#[test]
fn a_cancel_for_another_request_leaves_a_call_alone() {
    let call = json!({
        "jsonrpc": "2.0", "id": 3, "method": "tools/call",
        "params": { "name": "no_such_tool" }
    });
    let cancel = json!({
        "jsonrpc": "2.0", "method": "notifications/cancelled",
        "params": { "requestId": "3" }
    });
    let replies = exchange(&[&call.to_string(), &cancel.to_string()]);
    assert_eq!(replies.len(), 1, "{replies:?}");
    assert_eq!(replies[0]["id"], json!(3));
}
