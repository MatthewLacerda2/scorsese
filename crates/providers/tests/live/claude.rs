//! What Claude's answers mean, judged over the recorded streams the client's
//! own tests replay. Those are hand-written (each says so); the live check is
//! what replaces them.

use scorsese_providers::api::anthropic::content::Block;
use scorsese_providers::claude::{self, ClaudeError, Response};
use scorsese_providers::live::Verdict;
use scorsese_providers::live::claude::{TOOL, answer_step, cost, tool_turn_step};

use super::{fixture, refused};

/// A recorded stream, folded into a reply the way a live call is.
fn replayed(name: &str) -> Result<Response, ClaudeError> {
    claude::replay(
        fixture(&format!("anthropic/{name}")).as_bytes(),
        &mut |_| {},
    )
}

#[test]
fn a_tool_use_turn_is_ok_and_hands_back_what_to_replay() {
    let (step, called) = tool_turn_step(replayed("tool_use.sse"), "text_new", 2);
    assert_eq!(step.verdict, Verdict::Ok, "{step:?}");
    let (content, id) = called.unwrap();
    assert!(
        content.iter().any(|b| matches!(b, Block::Thinking { .. })),
        "thinking is replayed"
    );
    assert!(
        content
            .iter()
            .any(|b| matches!(b, Block::ToolUse { id: call, .. } if *call == id))
    );
}

#[test]
fn a_turn_that_never_calls_the_tool_asked_for_is_a_shape_change() {
    let (step, called) = tool_turn_step(replayed("tool_use.sse"), TOOL, 0);
    assert!(matches!(&step.verdict, Verdict::ShapeChanged { field } if field.contains(TOOL)));
    assert!(called.is_none());
}

/// Ending the turn instead of calling a tool is the model's choice, not the
/// API's shape moving: unfinished, and nothing to replay.
#[test]
fn a_turn_that_answers_instead_of_calling_is_unfinished() {
    let (step, called) = tool_turn_step(replayed("answer.sse"), TOOL, 0);
    assert!(
        matches!(step.verdict, Verdict::Unfinished { .. }),
        "{step:?}"
    );
    assert!(called.is_none());
}

#[test]
fn the_answer_after_the_replay_is_ok_when_it_has_words() {
    let step = answer_step(replayed("answer.sse"));
    assert_eq!(step.verdict, Verdict::Ok, "{step:?}");
}

#[test]
fn a_refusal_and_an_overloaded_api_are_refusals() {
    let step = answer_step(replayed("refusal.sse"));
    assert!(matches!(&step.verdict, Verdict::Refused { said } if said.contains("bio")));
    let step = answer_step(replayed("overloaded.sse"));
    assert!(matches!(&step.verdict, Verdict::Refused { said } if said.contains("overloaded")));
}

/// A stream cut before `message_stop` is the reply no longer being the
/// shape the client reads.
#[test]
fn a_stream_that_stops_early_is_a_shape_change() {
    let whole = fixture("anthropic/tool_use.sse");
    let cut = &whole[..whole.find("event: message_delta").unwrap()];
    let reply = claude::replay(cut.as_bytes(), &mut |_| {});
    let (step, _) = tool_turn_step(reply, "text_new", 0);
    assert!(
        matches!(&step.verdict, Verdict::ShapeChanged { field } if field.contains("message_stop"))
    );
}

/// Both bodies are hand-written from Anthropic's error reference.
#[test]
fn a_rejected_key_is_an_auth_failure_and_a_rejected_body_a_refusal() {
    let key =
        r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
    let step = answer_step(Err(ClaudeError::Http(refused(401, key))));
    assert!(
        matches!(step.verdict, Verdict::AuthFailed { .. }),
        "{step:?}"
    );

    let body =
        r#"{"type":"error","error":{"type":"invalid_request_error","message":"messages: system"}}"#;
    let step = answer_step(Err(ClaudeError::Http(refused(400, body))));
    assert!(
        matches!(&step.verdict, Verdict::Refused { said } if said.contains("messages: system"))
    );
}

/// Two calls, each bounded by its reply's token cap and a generous input,
/// priced as the dearest input there is.
#[test]
fn the_quote_is_a_ceiling_of_a_few_cents() {
    assert_eq!(cost(), 15);
}
