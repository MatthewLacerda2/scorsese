//! Gemini's signatures stay on the parts they came on, and a reply with no
//! words is still one that can be sent back.

use scorsese_providers::chat::Stop;
use serde_json::{Value, json};

use super::replay;

#[test]
fn a_signed_thought_keeps_its_signature_and_a_signed_part_is_not_joined_onto() {
    let (reply, heard) = replay("chat-signed-thought.sse");
    let reply = reply.unwrap();
    assert_eq!(heard[0], "progress:Weighing it.");
    let native: Value = serde_json::from_str(reply.native.get()).unwrap();
    assert_eq!(
        native["parts"],
        json!([
            {"text": "", "thoughtSignature": "dGhvdWdodA=="},
            {"text": "One ", "thoughtSignature": "b25l"},
            {"text": "two."},
        ])
    );
    assert_eq!(reply.text(), "One \n\ntwo.");
}

#[test]
fn a_reply_with_no_parts_is_sent_back_as_an_empty_part_and_says_nothing() {
    let (reply, _) = replay("chat-empty.sse");
    let reply = reply.unwrap();
    assert_eq!(reply.stop, Stop::EndTurn);
    let native: Value = serde_json::from_str(reply.native.get()).unwrap();
    assert_eq!(native, json!({"role": "model", "parts": [{"text": ""}]}));
    assert!(reply.message.parts().is_empty(), "{:?}", reply.message);
}
