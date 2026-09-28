//! One turn, run to its end: call the model, charge the call, run the tools
//! it asks for, send their results back — until it answers, or something
//! says stop.
//!
//! Runs as a task of its own, started by [`super::start`] after the turn's
//! row is committed; whoever asked has already been answered with the turn,
//! and follows it on the event stream.

use std::sync::Arc;
use std::time::Duration;

use scorsese_providers::api::anthropic::request::{Message, MessageContent, Role};
use scorsese_providers::claude::{self, Claude, ClaudeError, Response, Stop};
use serde_json::value::RawValue;

use super::relay::Relay;
use super::store::turns::{self, Charge};
use super::{calls, prompt};
use crate::credits::dollars;
use crate::db::UserId;
use crate::events::Event;
use crate::http::AppState;

/// How long to wait before sending a failed call again, each time.
const RETRY_AFTER: [Duration; 2] = [Duration::from_secs(2), Duration::from_secs(8)];

/// Everything a running turn needs.
pub(super) struct Running {
    /// Whose it is.
    pub(super) user: UserId,
    /// The turn.
    pub(super) turn: i64,
    /// The project its conversation is about.
    pub(super) project: i64,
    /// What the user wrote.
    pub(super) prompt: String,
    /// The client.
    pub(super) claude: Arc<dyn Claude>,
    /// Every earlier turn's messages.
    pub(super) history: Vec<Box<RawValue>>,
    /// This turn's, so far.
    pub(super) messages: Vec<Box<RawValue>>,
    /// Their balance when it started.
    pub(super) balance: i64,
}

/// How a turn ended: its state, and what to tell the user.
struct End(&'static str, String);

/// Run `turn` to its end, and record and announce how it ended.
pub(super) async fn run(state: AppState, mut turn: Running) {
    let end = match drive(&state, &mut turn).await {
        Ok(end) => end,
        Err(failure) => End("failed", failure),
    };
    state.assistant.stop_requested(turn.turn);
    match turns::finish(&state.pool, turn.user, turn.turn, end.0, &end.1).await {
        Ok((view, balance)) => state.events.send(
            turn.user,
            Event::ChatTurn {
                turn: view,
                balance_micros: balance,
            },
        ),
        Err(error) => eprintln!(
            "scorsese-server: assistant: ending turn {}: {error}",
            turn.turn
        ),
    }
}

/// The loop. `Err` is a failure, in words for the user.
async fn drive(state: &AppState, turn: &mut Running) -> Result<End, String> {
    let assistant = &state.assistant;
    let tools = prompt::tools(&state.tools);
    let (mut spent, mut balance) = (0, turn.balance);
    let mut revision = calls::revision(state, turn.user, turn.project).await;
    loop {
        if assistant.stop_requested(turn.turn) {
            return Ok(End("stopped", "Stopped, as you asked.".into()));
        }
        if spent >= assistant.cap_micros() {
            return Ok(End(
                "capped",
                format!(
                    "Stopped: this message has cost {}, the most one message may. Send another to \
                 carry on.",
                    dollars(spent)
                ),
            ));
        }
        if balance <= 0 {
            return Ok(End(
                "capped",
                format!(
                    "Stopped: your balance is {}. Add credit to carry on.",
                    dollars(balance)
                ),
            ));
        }
        let mut messages = turn.history.clone();
        messages.extend(turn.messages.iter().cloned());
        let request = claude::request(&assistant.settings, prompt::SYSTEM, tools.clone(), messages);
        let reply = ask(state, turn, request).await.map_err(|error| {
            eprintln!("scorsese-server: assistant: turn {}: {error}", turn.turn);
            format!("The assistant could not be reached: {error}")
        })?;
        let charge = Charge {
            turn: turn.turn,
            project: turn.project,
            prompt: &turn.prompt,
            model: &assistant.settings.model,
        };
        let (charged, now) = turns::charge(&state.pool, turn.user, &charge, &reply)
            .await
            .map_err(|error| error.to_string())?;
        (spent, balance) = (spent + charged, now);
        announce(state, turn, balance).await;
        match &reply.stop {
            Stop::EndTurn => {
                keep(state, turn, assistant_message(&reply)?).await?;
                return Ok(End("answered", reply.text()));
            }
            Stop::ToolUse => {
                keep(state, turn, assistant_message(&reply)?).await?;
                let mut results = Vec::new();
                for call in reply.calls() {
                    results.push(calls::run(state, turn.user, turn.turn, &call).await);
                }
                let now = calls::revision(state, turn.user, turn.project).await;
                if let Some(now) = now.filter(|now| Some(*now) != revision) {
                    let id = turn.project;
                    state
                        .events
                        .send(turn.user, Event::Project { id, revision: now });
                }
                revision = now;
                let results = Message {
                    role: Role::User,
                    content: MessageContent::Blocks(results),
                };
                keep(state, turn, results.raw().map_err(|e| e.to_string())?).await?;
            }
            Stop::Refusal { explanation, .. } => {
                let why = explanation.as_deref().unwrap_or("no reason was given");
                return Ok(End(
                    "refused",
                    format!("Claude declined this request: {why}"),
                ));
            }
            Stop::MaxTokens => {
                return Ok(End(
                    "failed",
                    "The reply ran past its length limit. Ask for \
                                         less at once, or send another message to carry on."
                        .into(),
                ));
            }
            Stop::Other(reason) => {
                return Ok(End(
                    "failed",
                    format!("The reply stopped early ({reason})."),
                ));
            }
        }
    }
}

/// Send `request`, retrying a failure worth retrying while nothing of the
/// reply has reached the browser yet.
async fn ask(
    state: &AppState,
    turn: &Running,
    request: scorsese_providers::api::anthropic::request::Request,
) -> Result<Response, ClaudeError> {
    let request = Arc::new(request);
    let mut waits = RETRY_AFTER.iter();
    loop {
        let (claude, request_) = (turn.claude.clone(), request.clone());
        let mut relay = Relay::new(state.events.clone(), turn.user, turn.turn);
        let (reply, heard) = tokio::task::spawn_blocking(move || {
            let reply = claude.reply(&request_, &mut |piece| relay.hear(piece));
            relay.flush();
            (reply, relay.heard)
        })
        .await
        .map_err(|_| ClaudeError::Cut)?;
        match (reply, waits.next()) {
            (Err(error), Some(wait)) if error.retryable() && !heard => {
                eprintln!(
                    "scorsese-server: assistant: turn {}: {error}; retrying",
                    turn.turn
                );
                tokio::time::sleep(*wait).await;
            }
            (reply, _) => return reply,
        }
    }
}

/// The model's reply as the message sent back next call, unchanged.
fn assistant_message(reply: &Response) -> Result<Box<RawValue>, String> {
    Message {
        role: Role::Assistant,
        content: MessageContent::Blocks(reply.content.clone()),
    }
    .raw()
    .map_err(|error| error.to_string())
}

/// Add `message` to the turn, and keep it.
async fn keep(state: &AppState, turn: &mut Running, message: Box<RawValue>) -> Result<(), String> {
    turn.messages.push(message);
    turns::keep(&state.pool, turn.user, turn.turn, &turn.messages)
        .await
        .map_err(|error| {
            eprintln!(
                "scorsese-server: assistant: keeping turn {}: {error}",
                turn.turn
            );
            "The conversation could not be saved.".to_owned()
        })
}

/// Tell the browser what the turn has cost so far, and the balance.
async fn announce(state: &AppState, turn: &Running, balance: i64) {
    match super::store::view_of(&state.pool, turn.user, turn.turn).await {
        Ok(view) => state.events.send(
            turn.user,
            Event::ChatTurn {
                turn: view,
                balance_micros: balance,
            },
        ),
        Err(error) => eprintln!("scorsese-server: assistant: turn {}: {error}", turn.turn),
    }
}
