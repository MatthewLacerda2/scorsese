//! A recorded stream is folded into the reply it carries.

use scorsese_providers::api::anthropic::content::Block;
use scorsese_providers::claude::{Stop, Streamed};
use scorsese_providers::prices::chat::Usage;
use serde_json::json;

use super::replay;

#[test]
fn a_reply_that_calls_tools_keeps_every_block_in_order() {
    let (reply, heard) = replay("tool_use.sse");
    let reply = reply.unwrap();
    assert_eq!(reply.id, "msg_01XFDUDYJgAACzvnptvVoYEL");
    assert_eq!(reply.model, "claude-opus-5-5");
    assert_eq!(reply.stop, Stop::ToolUse);
    let kinds: Vec<&str> = reply
        .content
        .iter()
        .map(|block| match block {
            Block::Thinking { .. } => "thinking",
            Block::Text { .. } => "text",
            Block::ToolUse { .. } => "tool_use",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["thinking", "text", "tool_use", "tool_use"]);
    let Block::Thinking {
        thinking,
        signature,
    } = &reply.content[0]
    else {
        unreachable!()
    };
    assert_eq!(
        thinking,
        "Checking what the project holds before placing the title."
    );
    assert!(signature.starts_with("EqQB"), "{signature}");
    assert_eq!(reply.text(), "Adding the title card now.");
    assert_eq!(
        heard,
        [
            "progress:Checking what the project holds ",
            "progress:before placing the title.",
            "end",
            "text:Adding the title ",
            "text:card now.",
            "end",
            "end",
            "end",
        ]
    );
}

#[test]
fn a_tool_calls_arguments_are_the_exact_text_the_model_wrote() {
    let reply = replay("tool_use.sse").0.unwrap();
    let Block::ToolUse { input, .. } = &reply.content[2] else {
        unreachable!()
    };
    // Spacing and key order as streamed, not as a map would re-serialise it.
    assert_eq!(
        input.get(),
        r#"{"project": 12, "text": "Minha viagem", "at": 0.5}"#
    );
    let calls = reply.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].name, "text_new");
    assert_eq!(
        calls[0].input,
        json!({"project": 12, "text": "Minha viagem", "at": 0.5})
    );
    assert_eq!(calls[1].id, "toolu_01B7gDzN8dmkTqTjXh8f1zwq");
    assert_eq!(calls[1].input, json!({}), "no fragments means no arguments");
}

#[test]
fn usage_is_counted_by_the_kind_each_token_is_billed_as() {
    let reply = replay("tool_use.sse").0.unwrap();
    assert_eq!(
        reply.usage,
        Usage {
            input: 412,
            output: 214,
            cache_write_5m: 0,
            cache_write_1h: 18_230,
            cache_read: 0,
        },
        "output from message_delta, the rest from message_start"
    );
    let answer = replay("answer.sse").0.unwrap();
    assert_eq!(answer.usage.cache_read, 18_230);
    assert_eq!(answer.usage.cache_write_5m, 1_507);
}

#[test]
fn an_answer_ends_the_turn_and_an_empty_progress_note_says_nothing() {
    let (reply, heard) = replay("answer.sse");
    let reply = reply.unwrap();
    assert_eq!(reply.stop, Stop::EndTurn);
    assert_eq!(
        reply.text(),
        "Done: the title card opens the video, half a second in."
    );
    assert!(reply.calls().is_empty());
    assert!(!heard.iter().any(|piece| piece.starts_with("progress:")));
}

#[test]
fn a_refusal_is_a_reply_with_its_reason_not_an_error() {
    let reply = replay("refusal.sse").0.unwrap();
    assert_eq!(
        reply.stop,
        Stop::Refusal {
            category: Some("bio".into()),
            explanation: Some("This request was declined by a safety classifier.".into()),
        }
    );
    assert!(reply.content.is_empty());
    assert_eq!(reply.usage.cache_read, 19_737);
}

#[test]
fn an_error_partway_is_an_error_worth_retrying() {
    let error = replay("overloaded.sse").0.unwrap_err();
    assert!(error.retryable(), "{error}");
    assert!(error.to_string().contains("overloaded_error"), "{error}");
}

#[test]
fn a_stream_that_stops_early_is_cut_not_a_reply() {
    let whole = super::fixture("answer.sse");
    let half = &whole[..whole.find("event: message_delta").unwrap()];
    let reply = scorsese_providers::claude::replay(half.as_bytes(), &mut |_: Streamed<'_>| {});
    let error = reply.unwrap_err();
    assert!(error.retryable(), "{error}");
}
