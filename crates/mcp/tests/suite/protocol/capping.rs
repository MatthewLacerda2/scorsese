//! At most ten pictures a reply (#1435): `still` says so in its schema and
//! refuses a longer list with how to split it, while `project_describe`,
//! which answers in words, takes the same `at` uncapped.

use super::fixture::project;
use crate::protocol::{call, once, said};
use serde_json::{Value, json};

/// `count` instants, a frame apart.
fn instants(count: usize) -> Value {
    (0..count).map(|frame| json!(frame.to_string())).collect()
}

#[test]
fn still_shows_at_most_ten_instants_a_call_and_says_how_to_see_more() {
    let reply = once(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }).to_string());
    let tools = reply["result"]["tools"].as_array().expect("a list");
    let schema = |name: &str| {
        let tool = tools.iter().find(|tool| tool["name"] == name).expect(name);
        tool["inputSchema"]["properties"]["at"].clone()
    };
    assert_eq!(schema("still")["maxItems"], 10);
    assert_eq!(schema("project_describe")["maxItems"], Value::Null);

    let dir = project("capping");
    let asked = json!({ "project": dir, "at": instants(11), "resolution": "160x90" });
    let (text, failed) = said(&call("still", asked));
    assert!(failed, "eleven pictures are refused: {text}");
    assert!(
        text.contains("at most 10") && text.contains("Split"),
        "{text}"
    );
    let (text, failed) = said(&call(
        "project_describe",
        json!({ "project": dir, "at": instants(11) }),
    ));
    assert!(!failed, "words have no cap: {text}");
    std::fs::remove_dir_all(dir).ok();
}
