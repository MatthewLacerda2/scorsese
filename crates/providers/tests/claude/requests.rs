//! A request is laid out for caching, and a conversation replays byte for byte.

use scorsese_providers::api::anthropic::content::{Block, ResultPart};
use scorsese_providers::api::anthropic::request::{Effort, Message, MessageContent, Role, Tool};
use scorsese_providers::claude::{self, MAX_TOKENS, MODEL, Settings};
use serde_json::{Value, json};

use super::replay;

fn settings() -> Settings {
    Settings {
        model: MODEL.to_owned(),
        effort: Effort::High,
        max_tokens: MAX_TOKENS,
    }
}

fn tool() -> Tool {
    Tool {
        name: "icons".into(),
        description: "The icon names.".into(),
        input_schema: json!({"type": "object", "properties": {}}),
    }
}

#[test]
fn the_prefix_is_cached_for_an_hour_and_the_tail_automatically() {
    let first = Message {
        role: Role::User,
        content: MessageContent::Text("hello".into()),
    };
    let request = claude::request(
        &settings(),
        "You edit videos.",
        vec![tool()],
        vec![first.raw().unwrap()],
    );
    let sent: Value = serde_json::to_value(&request).unwrap();
    assert_eq!(sent["model"], "claude-opus-5-5");
    assert_eq!(sent["stream"], true);
    assert_eq!(sent["max_tokens"], 64_000);
    assert_eq!(
        sent["thinking"],
        json!({"type": "adaptive", "display": "updates"})
    );
    assert_eq!(sent["output_config"], json!({"effort": "high"}));
    assert_eq!(
        sent["cache_control"],
        json!({"type": "ephemeral", "ttl": "5m"})
    );
    assert_eq!(
        sent["system"],
        json!([{"type": "text", "text": "You edit videos.",
                "cache_control": {"type": "ephemeral", "ttl": "1h"}}])
    );
    assert_eq!(
        sent["tools"],
        json!([{"name": "icons", "description": "The icon names.",
                "input_schema": {"type": "object", "properties": {}}}])
    );
    assert_eq!(
        sent["messages"],
        json!([{"role": "user", "content": "hello"}])
    );
    assert!(
        sent.get("tool_choice").is_none(),
        "forced tool use is a 400"
    );
}

#[test]
fn a_reply_is_sent_back_exactly_as_it_arrived() {
    let reply = replay("tool_use.sse").0.unwrap();
    let frozen = Message {
        role: Role::Assistant,
        content: MessageContent::Blocks(reply.content.clone()),
    }
    .raw()
    .unwrap();
    let text = frozen.get();
    assert!(
        text.contains(r#""input":{"project": 12, "text": "Minha viagem", "at": 0.5}"#),
        "the model's own bytes, not a re-serialisation: {text}"
    );
    assert!(
        text.contains(r#""signature":"EqQBCgIYAhIM1gbcDa9GJwZA2b3hGgxRrjvS5ZbBDfn6a7wiMK7r3v""#)
    );
    // Frozen once, the same text goes out in every later request.
    let request = claude::request(&settings(), "p", vec![], vec![frozen.clone()]);
    let body = serde_json::to_string(&request).unwrap();
    assert!(body.contains(text), "{body}");
}

#[test]
fn a_tool_result_carries_words_and_pictures_and_says_when_it_refused() {
    let answered = Block::ToolResult {
        tool_use_id: "toolu_1".into(),
        content: vec![
            ResultPart::Text {
                text: "frame at 2s".into(),
            },
            ResultPart::png("iVBORw0KGgo="),
        ],
        is_error: false,
    };
    let refused = Block::ToolResult {
        tool_use_id: "toolu_2".into(),
        content: vec![ResultPart::Text {
            text: "no such clip".into(),
        }],
        is_error: true,
    };
    let message: Value = serde_json::from_str(
        Message {
            role: Role::User,
            content: MessageContent::Blocks(vec![answered, refused]),
        }
        .raw()
        .unwrap()
        .get(),
    )
    .unwrap();
    assert_eq!(
        message,
        json!({"role": "user", "content": [
            {"type": "tool_result", "tool_use_id": "toolu_1", "content": [
                {"type": "text", "text": "frame at 2s"},
                {"type": "image", "source": {"type": "base64", "media_type": "image/png",
                                             "data": "iVBORw0KGgo="}}]},
            {"type": "tool_result", "tool_use_id": "toolu_2", "is_error": true,
             "content": [{"type": "text", "text": "no such clip"}]}]})
    );
}

#[test]
fn an_operators_note_is_a_system_message_of_plain_text() {
    let note = Message {
        role: Role::System,
        content: MessageContent::Text("The project is 12.".into()),
    };
    assert_eq!(
        note.raw().unwrap().get(),
        r#"{"role":"system","content":"The project is 12."}"#
    );
}
