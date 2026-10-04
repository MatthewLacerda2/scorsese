//! Running the tools the model asks for, and handing their answers back.
//!
//! Each call goes to the same [`Toolbox`](crate::tools::Toolbox) web MCP
//! serves, as the assistant's, in the turn it was made in. What happens here
//! is only what sits between a model and a paid tool (the module doc of
//! [`super`] argues it): a call naming `confirm` is refused without running,
//! and a quote a call issues is taken out of what the model reads and held for
//! the user instead.

use scorsese_mcp::Reply;
use scorsese_providers::chat::{Call, Part, ResultPart};

use super::store::QuoteView;
use super::store::turns::hold_quote;
use crate::credits::{from_cents, price};
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

/// Run `call`, made in turn `turn`, for `user`: its result, as the record
/// keeps it.
pub(super) async fn run(state: &AppState, user: UserId, turn: i64, call: &Call) -> Part {
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
    let outcome = if call.input.get("confirm").is_some() {
        let why = NOT_YOURS.to_owned();
        state
            .tools
            .refuse(user, client, &call.name, &call.input, why)
            .await
    } else {
        let (id, outcome) = state
            .tools
            .recorded(user, client, &call.name, &call.input)
            .await;
        match (id, outcome) {
            (Some(id), Ok(reply)) => Ok(held(state, user, turn, (id, &call.name), reply).await),
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
    Part::Result {
        call: call.id.clone(),
        name: call.name.clone(),
        content: blocks(parts),
        is_error,
    }
}

/// A reply's parts as a tool result's: words, then the picture they are
/// about. Claude refuses an empty text block, so empty words are dropped.
fn blocks(parts: Vec<scorsese_mcp::Part>) -> Vec<ResultPart> {
    let mut blocks = Vec::new();
    for part in parts {
        if !part.text.trim().is_empty() {
            blocks.push(ResultPart::Text { text: part.text });
        }
        if let Some(png) = part.image {
            blocks.push(ResultPart::Png { data: png });
        }
    }
    if blocks.is_empty() {
        blocks.push(ResultPart::Text {
            text: "(no words)".into(),
        });
    }
    blocks
}

/// `reply` as the model may read it: when the call issued a quote, the quote
/// is held on the turn and shown to the user, and the line carrying its token
/// is cut from the words.
async fn held(state: &AppState, user: UserId, turn: i64, call: (i64, &str), reply: Reply) -> Reply {
    let (call, tool) = call;
    let pending = match state.tools.pending_quote(user, call).await {
        Ok(Some(pending)) => pending,
        Ok(None) => return reply,
        Err(error) => {
            // Unknowable whether a quote was issued, so nothing of the reply
            // may reach the model: it could carry a token.
            eprintln!("scorsese-server: assistant: reading a quote: {error}");
            return "A quote could not be held for the person; nothing was spent. Try again."
                .into();
        }
    };
    let lines: Vec<String> = reply
        .parts
        .iter()
        .flat_map(|part| part.text.lines())
        .filter(|line| !line.contains(&pending.token))
        .map(str::to_owned)
        .collect();
    let quote = QuoteView {
        tool: tool.to_owned(),
        lines: lines.clone(),
        micros: price(from_cents(pending.cents)),
        expires_at: pending.expires_at,
    };
    if let Err(error) = hold_quote(&state.pool, user, turn, &pending.token, &quote).await {
        eprintln!("scorsese-server: assistant: holding a quote: {error}");
        return "A quote could not be held for the person; nothing was spent. Try again.".into();
    }
    state.events.send(user, Event::ChatQuote { turn, quote });
    let mut words = lines;
    words.push(
        "This quote is now in front of the person as a confirmation box; you do not have its \
         token and cannot confirm it. Nothing has been spent. End your turn now: say in one line \
         what it covers and what it costs, and wait — their answer arrives as a system message."
            .to_owned(),
    );
    words.join("\n").into()
}

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
