//! Running the tools the model asks for, and handing their answers back.
//!
//! Each call goes to the same [`Toolbox`](crate::tools::Toolbox) web MCP
//! serves, as the assistant's, in the turn it was made in. What happens here
//! is only what sits between a model and a paid tool (the module doc of
//! [`super`] argues it): a call naming `confirm` is refused without running,
//! and a quote a call issues is taken out of what the model reads and held for
//! the user instead — which pauses the turn until they answer (#1440), their
//! answer becoming that call's result ([`settle`]).

use scorsese_mcp::Reply;
use scorsese_providers::chat::{Call, Message, Part, ResultPart};
use scorsese_render::Tools;

use super::store::QuoteView;
use super::store::turns::hold_quote;
use super::{described, pictures};
use crate::db::{self, UserId};
use crate::events::Event;
use crate::http::AppState;
use crate::tools::Client;

/// How much of a tool's answer the browser is shown as it happens; the
/// whole of it is in the tool-call log.
const SAID: usize = 300;

/// What the model is told when it tries to spend on its own word.
const NOT_YOURS: &str = "Refused: `confirm` is the person's to give, never yours. Call the \
tool without confirm; they are shown the quote and answer it themselves.";

/// What a quoting call's result says while the person is asked; their answer
/// takes its place before the model reads it ([`settle`]).
const HELD: &str = "This quote is in front of the person as a confirmation box, and the turn \
waits for their answer. Nothing has been spent.";

/// What a quote another quote in the same reply took the place of says.
const SUPERSEDED: &str = "Not shown: a later call in the same reply quoted too, and only one \
quote waits for an answer at a time. This one was withdrawn and nothing was spent; put every \
paid item in one call.";

/// Run `call`, made in turn `turn`, for `user`: its result, as the record
/// keeps it, and whether it held a quote for the person — which pauses the
/// turn.
pub(super) async fn run(state: &AppState, user: UserId, turn: i64, call: &Call) -> (Part, bool) {
    let tell = |state_: &'static str, said: Option<String>| {
        state.events.send(
            user,
            Event::ChatTool {
                turn,
                tool: call.name.clone(),
                state: state_,
                said,
            },
        );
    };
    tell("running", None);
    let client = Client::Assistant { turn };
    let mut quoted = false;
    let outcome = if call.input.get("confirm").is_some() {
        let why = NOT_YOURS.to_owned();
        state
            .tools
            .refuse(user, client, &call.name, &call.input, why)
            .await
    } else {
        // A Stop asked for while it runs stops it too (#1469).
        let calling = state.assistant.stopping.calling(turn);
        let (id, outcome) = state
            .tools
            .recorded(user, client, &call.name, &call.input, &calling.cancel)
            .await;
        drop(calling);
        match (id, outcome) {
            (Some(id), Ok(reply)) => {
                let (reply, held) = held(state, user, turn, (id, call), reply).await;
                quoted = held;
                Ok(reply)
            }
            (_, outcome) => outcome,
        }
    };
    let (parts, is_error) = match outcome {
        Ok(reply) => (reply.parts, false),
        Err(refusal) => (Reply::from(refusal).parts, true),
    };
    let words: Vec<&str> = parts.iter().map(|part| part.text.as_str()).collect();
    let said: String = words.join("\n").chars().take(SAID).collect();
    tell(if is_error { "refused" } else { "answered" }, Some(said));
    let ffmpeg = state.tools.ffmpeg().clone();
    let content = tokio::task::spawn_blocking(move || blocks(&ffmpeg, parts))
        .await
        .unwrap_or_else(|_| {
            vec![ResultPart::Text {
                text: "(the answer could not be read back)".into(),
            }]
        });
    let part = Part::Result {
        call: call.id.clone(),
        name: call.name.clone(),
        content,
        is_error,
    };
    (part, quoted)
}

/// Put `answer` — the result the person's answer to a quote makes — in place
/// of its call's held result in `results`, the paused turn's last message,
/// and withdraw any other quote held in the same reply. The model has not
/// read that message yet, so nothing it has seen is edited. Whether the call
/// was found.
pub(super) fn settle(results: &mut Message, answer: Part) -> bool {
    let (Message::User { content }, Part::Result { call: answered, .. }) = (&mut *results, &answer)
    else {
        return false;
    };
    let answered = answered.clone();
    let mut answer = Some(answer);
    for part in content.iter_mut() {
        let Part::Result { call, content, .. } = part else {
            continue;
        };
        if *call == answered {
            if let Some(answer) = answer.take() {
                *part = answer;
            }
        } else if content.iter().any(held_by) {
            *content = vec![ResultPart::Text {
                text: SUPERSEDED.to_owned(),
            }];
        }
    }
    answer.is_none()
}

/// Whether `block` is a result held for the person's answer.
fn held_by(block: &ResultPart) -> bool {
    matches!(block, ResultPart::Text { text } if text.ends_with(HELD))
}

/// A reply's parts as a tool result's: words, then the picture they are
/// about. Claude refuses an empty text block, so empty words are dropped.
///
/// Each picture is sent, and so stored and replayed with every later turn,
/// as a JPEG no longer than the model reads ([`pictures`]).
fn blocks(ffmpeg: &Tools, parts: Vec<scorsese_mcp::Part>) -> Vec<ResultPart> {
    let mut blocks = Vec::new();
    for part in parts {
        if !part.text.trim().is_empty() {
            blocks.push(ResultPart::Text { text: part.text });
        }
        if let Some(png) = part.image {
            blocks.push(pictures::shrunk(ffmpeg, &png));
        }
    }
    if blocks.is_empty() {
        blocks.push(ResultPart::Text {
            text: "(no words)".into(),
        });
    }
    blocks
}

/// `reply` as the model may read it, and whether it held a quote: when the
/// call issued one, the quote is held on the turn and shown to the user, and
/// the line carrying its token is cut from the words.
async fn held(
    state: &AppState,
    user: UserId,
    turn: i64,
    call: (i64, &Call),
    reply: Reply,
) -> (Reply, bool) {
    let (row, call) = call;
    let pending = match state.tools.pending_quote(user, row).await {
        Ok(Some(pending)) => pending,
        Ok(None) => return (reply, false),
        Err(error) => {
            // Unknowable whether a quote was issued, so nothing of the reply
            // may reach the model: it could carry a token.
            tracing::error!("assistant: reading a quote: {error}");
            return (UNHELD.into(), false);
        }
    };
    let lines: Vec<String> = reply
        .parts
        .iter()
        .flat_map(|part| part.text.lines())
        .filter(|line| !line.contains(&pending.token))
        .map(str::to_owned)
        .collect();
    let (items, rest) = described::describe(state, user, &call.name, &call.input, &lines).await;
    let quote = QuoteView {
        tool: call.name.clone(),
        items,
        lines: rest,
        micros: pending.micros,
        expires_at: pending.expires_at,
        batched: call.input.get("batch").and_then(serde_json::Value::as_bool) == Some(true),
        batch_micros: pending.batch_micros,
        call: Some(call.id.clone()),
    };
    match hold_quote(&state.pool, user, turn, &pending.token, &quote).await {
        // A second quote in one reply takes the first one's place.
        Ok(Some(earlier)) => {
            if let Err(error) = state.tools.withdraw_quote(user, &earlier).await {
                tracing::error!("assistant: withdrawing a quote: {error}");
            }
        }
        Ok(None) => {}
        Err(error) => {
            tracing::error!("assistant: holding a quote: {error}");
            return (UNHELD.into(), false);
        }
    }
    state.events.send(user, Event::ChatQuote { turn, quote });
    let mut words = lines;
    words.push(HELD.to_owned());
    (words.join("\n").into(), true)
}

/// What the model reads when a quote could not be held.
const UNHELD: &str = "A quote could not be held for the person; nothing was spent. Try again.";

/// `project`'s revision now, if it is `user`'s.
pub(super) async fn revision(state: &AppState, user: UserId, project: i64) -> Option<i64> {
    let read = async {
        let mut tx = db::scoped(&state.pool, user).await?;
        let revision: Option<i64> =
            sqlx::query_scalar("SELECT revision FROM projects WHERE id = $1")
                .bind(project)
                .fetch_optional(&mut *tx)
                .await?;
        tx.commit().await?;
        Ok::<_, sqlx::Error>(revision)
    };
    read.await.ok().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(call: &str, text: &str) -> Part {
        Part::Result {
            call: call.into(),
            name: "generate".into(),
            content: vec![ResultPart::Text { text: text.into() }],
            is_error: false,
        }
    }

    #[test]
    fn the_answer_takes_its_calls_place_and_a_quote_it_replaced_is_withdrawn() {
        let held = format!("vo: $0.60\n{HELD}");
        let looked = result("look", "The timeline is 30 seconds.");
        let mut results = Message::User {
            content: vec![
                result("first", &held),
                looked.clone(),
                result("second", &held),
            ],
        };
        let answer = result("second", "The person confirmed the quote: queued as job 4.");
        assert!(settle(&mut results, answer.clone()));
        assert_eq!(
            results,
            Message::User {
                content: vec![result("first", SUPERSEDED), looked, answer]
            }
        );
        assert!(
            !settle(&mut results, result("elsewhere", "?")),
            "no such call"
        );
    }
}
