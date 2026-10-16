//! The neutral record in the wire, and a history that survives a change of
//! model: Sonnet ↔ Opus, the only switch left since #1220.

use scorsese_providers::chat::{Kept, Message, Model, Part, ResultPart, freeze, replay};
use serde_json::value::RawValue;
use serde_json::{Value, json};

/// The last message's bytes, thinking block and all.
const THOUGHTFUL: &str = r#"{"role":"assistant","content":[{"type":"thinking","thinking":"","signature":"s"},{"type":"text","text":"Two icons."}]}"#;

/// A turn that called a tool and answered, as `model` ran it — its last
/// message carrying a thinking block, as a real Claude reply's bytes do.
fn on(model: Model) -> Kept {
    let record = vec![
        Message::user("what is in it?"),
        Message::System {
            text: "This is project 7.".into(),
        },
        Message::Assistant {
            content: vec![Part::Call {
                id: "toolu_1".into(),
                name: "icons".into(),
                input: json!({}),
            }],
        },
        Message::User {
            content: vec![Part::Result {
                call: "toolu_1".into(),
                name: "icons".into(),
                content: vec![
                    ResultPart::Text { text: "two".into() },
                    ResultPart::Png {
                        data: "UE5H".into(),
                    },
                ],
                is_error: false,
            }],
        },
        Message::assistant("Two icons."),
    ];
    let mut native: Vec<Box<RawValue>> = record
        .iter()
        .map(|message| freeze(model, message).expect("the test setup holds"))
        .collect();
    native[4] = RawValue::from_string(THOUGHTFUL.into()).expect("the test setup holds");
    Kept {
        model: model.id().into(),
        native,
        record,
    }
}

fn parsed(raw: &[Box<RawValue>]) -> Vec<Value> {
    raw.iter()
        .map(|message| serde_json::from_str(message.get()).expect("the test setup holds"))
        .collect()
}

fn texts(raw: &[Box<RawValue>]) -> Vec<String> {
    raw.iter().map(|m| m.get().to_owned()).collect()
}

#[test]
fn the_same_model_replays_the_bytes_it_was_sent() {
    for model in Model::ALL {
        let history = replay(model, &[on(model)]).unwrap();
        assert_eq!(history[4].get(), THOUGHTFUL, "thinking blocks stay");
    }
}

/// The two Claude models share a wire but not a signature: a switch between
/// them is translated from the record, so the other model's thinking blocks
/// are dropped rather than sent where they would be refused.
#[test]
fn a_switch_between_sonnet_and_opus_reads_the_record_both_ways() {
    for (from, to) in [
        (Model::ClaudeOpus55, Model::ClaudeSonnet55),
        (Model::ClaudeSonnet55, Model::ClaudeOpus55),
    ] {
        let history = parsed(&replay(to, &[on(from)]).unwrap());
        assert_eq!(history.len(), 5);
        assert_eq!(
            history[1],
            json!({"role": "system", "content": "This is project 7."})
        );
        assert_eq!(history[2]["content"][0]["type"], "tool_use");
        assert_eq!(history[3]["content"][0]["content"][1]["type"], "image");
        assert_eq!(
            history[4],
            json!({"role": "assistant", "content": [{"type": "text", "text": "Two icons."}]}),
            "{from:?} → {to:?} drops the thinking block"
        );
    }
}

#[test]
fn a_conversation_that_switched_keeps_each_turn_on_its_own_terms() {
    let turns = [on(Model::ClaudeOpus55), on(Model::ClaudeSonnet55)];
    let history = replay(Model::ClaudeSonnet55, &turns).unwrap();
    assert_eq!(history.len(), 10);
    assert_ne!(history[4].get(), THOUGHTFUL, "Opus's turn, translated");
    assert_eq!(history[9].get(), THOUGHTFUL, "Sonnet's own turn, as sent");
}

#[test]
fn translation_is_deterministic_so_the_cache_survives_the_second_turn() {
    let first = replay(Model::ClaudeSonnet55, &[on(Model::ClaudeOpus55)]).unwrap();
    let again = replay(Model::ClaudeSonnet55, &[on(Model::ClaudeOpus55)]).unwrap();
    assert_eq!(texts(&first), texts(&again));
}

/// The vendor's id is stored beside every charged call, so it is pinned: a
/// rename here would split one vendor's history in two.
#[test]
fn every_model_is_served_by_anthropic_under_that_name() {
    for model in Model::ALL {
        assert_eq!(model.vendor().id(), "anthropic");
    }
}
