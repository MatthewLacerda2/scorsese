//! Claude's part of the live check: one real tool-use turn, replayed.
//!
//! The client (#540) was written from Anthropic's reference with no key to
//! try it against, and three things it relies on had never met the real API
//! (#567's comment from #540):
//!
//! - **the stream it parses** — every fixture under `fixtures/anthropic/` is
//!   hand-written;
//! - **the `thinking-display-updates-2026-08-18` beta** — sent on every call,
//!   and the source of the web app's progress lines;
//! - **mid-conversation `role: "system"` messages** on `claude-opus-5-5`.
//!
//! So this is two streamed calls through the product's own request builder
//! ([`crate::claude::request`]) at `low` effort. The first asks for a call to
//! a one-argument tool; a `200` proves the beta header is accepted, and the
//! reply's events are the tool-use stream the fixtures imitate. The second
//! sends that reply back unchanged — thinking blocks and signatures included —
//! with the tool's result and then an operator note as a `system` message,
//! which is exactly how the server's assistant appends facts mid-turn. A
//! `400` there names what the API would not take.

use serde_json::{Value, json};

use crate::api::anthropic::content::{Block, ResultPart};
use crate::api::anthropic::request::{Effort, Message, MessageContent, Role, Tool};
use crate::api::tap::Tap;
use crate::claude::{
    self, Anthropic, Claude, ClaudeError, MODEL, Response, Settings, Stop, Streamed,
};
use crate::credentials::Secret;
use crate::prices::claude::{Usage, rate};
use crate::prices::dollars;

use super::{Step, Verdict, judge};

/// The most either reply may be. Small, because the answer is a word; not so
/// small that thinking at `low` effort runs into it.
pub const MAX_TOKENS: u32 = 2048;

/// A generous bound on the input of one call, in tokens — the prompt, the
/// tool, the tool-use preamble the API adds, and on the second call the first
/// reply. Priced as one-hour cache writes, the dearest input there is, so the
/// quote is a ceiling rather than a guess.
const INPUT_BOUND: u64 = 4_000;

/// The tool the model is asked to call.
pub const TOOL: &str = "echo";

/// What the tool is called with, and answers.
const WORD: &str = "scorsese";

/// The instructions.
const SYSTEM: &str = "You are checking that an API client works. Do exactly what you are asked, as briefly as possible.";

/// The operator's note, sent mid-conversation as a `system` message.
const NOTE: &str = "Reply with the word the tool returned and nothing else.";

/// What the Claude part of the check costs at most, in cents.
pub fn cost() -> u64 {
    let rate = rate(MODEL).expect("the assistant's model has a published rate");
    let per_call = Usage {
        cache_write_1h: INPUT_BOUND,
        output: u64::from(MAX_TOKENS),
        ..Usage::default()
    }
    .micros(rate);
    (per_call * 2).div_ceil(10_000)
}

/// The calls, as a quote lists them.
pub(super) fn calls() -> Vec<String> {
    vec![format!(
        "two streamed calls to {MODEL} at low effort — a tool call, then its result with a \
         mid-conversation system message — at most {}, billed by the tokens used",
        dollars(cost())
    )]
}

/// Runs the Claude part; returns what it found and what it spent.
pub(super) fn check(key: &Secret, tap: &Tap) -> (Vec<Step>, u64) {
    let client = Anthropic::new(key).tapped(tap);
    let settings = Settings {
        model: MODEL.to_owned(),
        effort: Effort::Low,
        max_tokens: MAX_TOKENS,
    };
    let ask = frozen(
        Role::User,
        MessageContent::Blocks(vec![Block::Text {
            text: format!("Call the {TOOL} tool with the word \"{WORD}\"."),
        }]),
    );
    let mut progress = 0;
    let first = client.reply(
        &claude::request(&settings, SYSTEM, vec![tool()], vec![ask.clone()]),
        &mut |piece| progress += usize::from(matches!(piece, Streamed::Progress(_))),
    );
    let mut micros = spent(&first);
    let (step, called) = tool_turn_step(first, TOOL, progress);
    let mut steps = vec![step];
    if let Some((content, id)) = called {
        let messages = vec![
            ask,
            frozen(Role::Assistant, MessageContent::Blocks(content)),
            frozen(
                Role::User,
                MessageContent::Blocks(vec![Block::ToolResult {
                    tool_use_id: id,
                    content: vec![ResultPart::Text {
                        text: WORD.to_owned(),
                    }],
                    is_error: false,
                }]),
            ),
            frozen(Role::System, MessageContent::Text(NOTE.to_owned())),
        ];
        let second = client.reply(
            &claude::request(&settings, SYSTEM, vec![tool()], messages),
            &mut |_| {},
        );
        micros += spent(&second);
        steps.push(answer_step(second));
    }
    (steps, micros.div_ceil(10_000))
}

/// A message as the JSON text it is sent as.
fn frozen(role: Role, content: MessageContent) -> Box<serde_json::value::RawValue> {
    Message { role, content }
        .raw()
        .expect("a message is plain data and serialises")
}

/// The tool: one string in.
fn tool() -> Tool {
    Tool {
        name: TOOL.to_owned(),
        description: String::from("Answers with the word it is given."),
        input_schema: json!({
            "type": "object",
            "properties": {"word": {"type": "string"}},
            "required": ["word"],
        }),
    }
}

/// What a reply cost, by the vendor's own token count; nothing if it failed.
fn spent(answer: &Result<Response, ClaudeError>) -> u64 {
    let Ok(response) = answer else { return 0 };
    rate(&response.model)
        .or_else(|| rate(MODEL))
        .map_or(0, |rate| response.usage.micros(rate))
}

/// What the first call said: whether it called `tool`, and if it did, the
/// reply to send back and the call's id.
pub fn tool_turn_step(
    answer: Result<Response, ClaudeError>,
    tool: &str,
    progress: usize,
) -> (Step, Option<(Vec<Block>, String)>) {
    let call = "POST messages — ask for a tool call (beta header on)";
    let response = match answer {
        Err(error) => return (Step::new(call, judge::claude(&error)), None),
        Ok(response) => response,
    };
    if let Some(verdict) = unfinished(&response, &Stop::ToolUse) {
        return (Step::new(call, verdict), None);
    }
    let Some(called) = response.calls().into_iter().find(|c| c.name == tool) else {
        let field =
            format!("content[].tool_use: stop_reason was tool_use and no {tool} call arrived");
        return (Step::new(call, Verdict::ShapeChanged { field }), None);
    };
    if !matches!(called.input, Value::Object(_)) {
        let field = format!("tool_use.input: expected an object, got {}", called.input);
        return (Step::new(call, Verdict::ShapeChanged { field }), None);
    }
    let thinking: Vec<&String> = response
        .content
        .iter()
        .filter_map(|block| match block {
            Block::Thinking { signature, .. } => Some(signature),
            _ => None,
        })
        .collect();
    if thinking.iter().any(|signature| signature.is_empty()) {
        let field = String::from("thinking.signature: a thinking block arrived unsigned");
        return (Step::new(call, Verdict::ShapeChanged { field }), None);
    }
    let step = Step::new(call, Verdict::Ok).noting(format!(
        "{} thinking block(s), {progress} progress piece(s) streamed — zero of either is fine at low effort",
        thinking.len()
    ));
    (step, Some((response.content, called.id)))
}

/// What the second call said: an answer in words, after the replay and the
/// mid-conversation system message were accepted.
pub fn answer_step(answer: Result<Response, ClaudeError>) -> Step {
    let call = "POST messages — tool result, thinking replayed, mid-conversation system message";
    let response = match answer {
        Err(error) => return Step::new(call, judge::claude(&error)),
        Ok(response) => response,
    };
    if let Some(verdict) = unfinished(&response, &Stop::EndTurn) {
        return Step::new(call, verdict);
    }
    match response.text() {
        text if text.trim().is_empty() => Step::new(
            call,
            Verdict::ShapeChanged {
                field: String::from("content[].text: the answer had no words"),
            },
        ),
        text => Step::new(call, Verdict::Ok).noting(format!("answered {text:?}")),
    }
}

/// Why a reply cannot be judged further, if it cannot: an id or model missing,
/// no tokens counted, a refusal, or a stop other than `expected`.
fn unfinished(response: &Response, expected: &Stop) -> Option<Verdict> {
    let usage = response.usage;
    let counted =
        usage.input + usage.output + usage.cache_read + usage.cache_write_5m + usage.cache_write_1h;
    if response.id.is_empty() || response.model.is_empty() {
        return Some(Verdict::ShapeChanged {
            field: String::from("message_start.message: id or model missing"),
        });
    }
    if counted == 0 {
        return Some(Verdict::ShapeChanged {
            field: String::from("usage: no tokens counted at either end of the stream"),
        });
    }
    match &response.stop {
        Stop::Refusal {
            category,
            explanation,
        } => Some(Verdict::Refused {
            said: format!(
                "a safety classifier declined it ({}): {}",
                category.as_deref().unwrap_or("no category"),
                explanation.as_deref().unwrap_or("no explanation")
            ),
        }),
        stop if stop == expected => None,
        stop => Some(Verdict::Unfinished {
            said: format!(
                "stopped with {} rather than {}",
                stop.as_str(),
                expected.as_str()
            ),
        }),
    }
}
