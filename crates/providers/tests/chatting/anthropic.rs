//! Claude behind the seam: a reply read as the neutral record, and the turns
//! kept before #705 — Anthropic's JSON alone — read back into one.

use std::collections::HashMap;

use scorsese_providers::chat::anthropic::{read, reply_of};
use scorsese_providers::chat::{Message, Part, ResultPart};
use scorsese_providers::claude;
use serde_json::json;
use serde_json::value::RawValue;

#[test]
fn a_claude_reply_keeps_its_words_and_calls_and_leaves_thinking_in_the_bytes() {
    let path = format!(
        "{}/fixtures/anthropic/tool_use.sse",
        env!("CARGO_MANIFEST_DIR")
    );
    let stream = std::fs::read_to_string(path).unwrap();
    let response = claude::replay(stream.as_bytes(), &mut |_| {}).unwrap();
    let reply = reply_of(response).unwrap();
    assert_eq!(reply.text(), "Adding the title card now.");
    let names: Vec<String> = reply.calls().into_iter().map(|call| call.name).collect();
    assert_eq!(names, ["text_new", "icons"]);
    assert!(
        reply.native.get().contains("\"thinking\""),
        "{}",
        reply.native.get()
    );
    let thoughtful = reply
        .message
        .parts()
        .iter()
        .all(|part| matches!(part, Part::Text { .. } | Part::Call { .. }));
    assert!(thoughtful);
}

fn raw(value: serde_json::Value) -> Box<RawValue> {
    RawValue::from_string(value.to_string()).expect("the test setup holds")
}

#[test]
fn a_turn_from_before_the_record_is_read_back_into_one() {
    let earlier = [
        raw(json!({"role": "user", "content": [{"type": "text", "text": "look"}]})),
        raw(json!({"role": "system", "content": "This is project 7."})),
        raw(json!({"role": "assistant", "content": [
            {"type": "thinking", "thinking": "", "signature": "s"},
            {"type": "tool_use", "id": "toolu_1", "name": "still", "input": {"at": 2}},
        ]})),
    ];
    let later = [
        raw(
            json!({"role": "user", "content": [{"type": "tool_result", "tool_use_id": "toolu_1",
            "content": [{"type": "text", "text": "a frame"},
                        {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "UE5H"}}],
            "is_error": true}]}),
        ),
        raw(json!({"role": "assistant", "content": "Done."})),
        raw(json!({"role": "user", "content": "thanks"})),
    ];
    let mut names = HashMap::new();
    let first = read(&earlier, &mut names);
    assert_eq!(
        first,
        [
            Message::user("look"),
            Message::System {
                text: "This is project 7.".into()
            },
            Message::Assistant {
                content: vec![Part::Call {
                    id: "toolu_1".into(),
                    name: "still".into(),
                    input: json!({"at": 2}),
                }]
            },
        ]
    );
    // The result is in the next turn; its tool's name came across with `names`.
    let second = read(&later, &mut names);
    assert_eq!(
        second,
        [
            Message::User {
                content: vec![Part::Result {
                    call: "toolu_1".into(),
                    name: "still".into(),
                    content: vec![
                        ResultPart::Text {
                            text: "a frame".into()
                        },
                        ResultPart::Png {
                            data: "UE5H".into()
                        },
                    ],
                    is_error: true,
                }]
            },
            Message::assistant("Done."),
            Message::user("thanks"),
        ]
    );
}
