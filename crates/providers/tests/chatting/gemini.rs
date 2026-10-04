//! Gemini's stream, folded.

use scorsese_providers::chat::gemini::GeminiError;
use scorsese_providers::chat::{Part, Stop, Usage};
use serde_json::{Value, json};

use super::replay;

#[test]
fn a_reply_that_calls_tools_keeps_words_calls_and_signatures() {
    let (reply, heard) = replay("chat-tool-call.sse");
    let reply = reply.unwrap();
    assert_eq!(reply.stop, Stop::ToolUse, "STOP with calls is a tool turn");
    assert_eq!(reply.model, "gemini-3.8-flash");
    assert_eq!(
        heard,
        [
            "progress:Looking at what is in the project first.",
            "end",
            "text:Let me ",
            "text:look.",
            "end",
            "end",
            "end",
        ]
    );
    let calls = reply.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].name, "project_read");
    assert_eq!(calls[0].input, json!({ "project": 7 }));
    assert_ne!(calls[0].id, calls[1].id, "made-up ids are unique");
    assert!(
        calls[0].id.starts_with("gemini_call_resp_Ab1_"),
        "{}",
        calls[0].id
    );
    assert_eq!(reply.text(), "Let me look.");

    // The bytes sent back: the words joined, the thought summary gone, the
    // signature on the call it came on.
    let native: Value = serde_json::from_str(reply.native.get()).unwrap();
    assert_eq!(
        native,
        json!({"role": "model", "parts": [
            {"text": "Let me look."},
            {"functionCall": {"name": "project_read", "args": {"project": 7}},
             "thoughtSignature": "c2lnLW9uZQ=="},
            {"functionCall": {"name": "icons", "args": {}}},
        ]})
    );
}

#[test]
fn cached_input_is_a_cache_read_and_thinking_is_output() {
    let (reply, _) = replay("chat-tool-call.sse");
    let reply = reply.unwrap();
    assert_eq!(reply.thinking, Some(200), "thoughtsTokenCount, said apart");
    let usage = reply.usage;
    assert_eq!(
        usage,
        Usage {
            input: 3_000,
            output: 240,
            cache_read: 9_000,
            ..Usage::default()
        }
    );
}

#[test]
fn an_answer_keeps_its_closing_signature() {
    let (reply, _) = replay("chat-answer.sse");
    let reply = reply.unwrap();
    assert_eq!(reply.stop, Stop::EndTurn);
    assert_eq!(reply.text(), "The project is empty.");
    let native: Value = serde_json::from_str(reply.native.get()).unwrap();
    assert_eq!(
        native["parts"],
        json!([{"text": "The project is empty.", "thoughtSignature": "c2lnLXR3bw=="}])
    );
    assert_eq!(
        reply.message.parts(),
        [Part::Text {
            text: "The project is empty.".into()
        }]
    );
}

#[test]
fn a_blocked_prompt_is_a_refusal_not_a_failure() {
    let (reply, _) = replay("chat-blocked.sse");
    let reply = reply.unwrap();
    assert_eq!(
        reply.stop,
        Stop::Refusal {
            category: Some("safety".into()),
            explanation: None
        }
    );
    assert_eq!(reply.usage.input, 50, "a refusal is still billed");
}

#[test]
fn a_stream_without_a_finish_is_cut_and_worth_retrying() {
    let (reply, _) = replay("chat-cut.sse");
    let error = reply.unwrap_err();
    assert!(matches!(error, GeminiError::Cut), "{error}");
    assert!(error.retryable());
}

#[test]
fn an_error_partway_names_googles_status() {
    let (reply, _) = replay("chat-error.sse");
    let error = reply.unwrap_err();
    assert!(error.to_string().contains("UNAVAILABLE"), "{error}");
    assert!(error.retryable());
}
