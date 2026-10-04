//! One turn, run to its end: call the model, charge the call, run the tools
//! it asks for, send their results back — until it answers, asks the user a
//! question ([`super::ask`]), or something says stop.
//!
//! Runs as a task of its own, started by [`super::start`] after the turn's
//! row is committed — or by [`super::ask::answer`], resuming a turn that
//! asked; whoever asked has already been answered with the turn, and follows
//! it on the event stream.

use std::sync::Arc;
use std::time::{Duration, Instant};

use scorsese_providers::chat::{self, Chat, ChatError, Message, Model, Reply, Request, Stop};
use serde_json::value::RawValue;

use super::relay::Relay;
use super::store::asking;
use super::store::turns::{self, Charge};
use super::store::{QuestionView, TurnView};
use super::{EFFORT, ask, calls, prompt};
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
    /// The model it runs on, start to end.
    pub(super) model: Model,
    /// That model's client.
    pub(super) chat: Arc<dyn Chat>,
    /// Every earlier turn's messages, in this model's wire.
    pub(super) history: Vec<Box<RawValue>>,
    /// This turn's, so far, as the model is sent them.
    pub(super) messages: Vec<Box<RawValue>>,
    /// The same, neutral.
    pub(super) record: Vec<Message>,
    /// Their balance when it started, or resumed.
    pub(super) balance: i64,
    /// What it had cost before then: nothing for a new turn, the calls made
    /// before its question for a resumed one. The per-turn cap covers both.
    pub(super) spent: i64,
}

/// How a turn ended: its state, and what to tell the user — or that it is
/// waiting on a question.
enum End {
    /// Over, in this state, with these words.
    Over(&'static str, String),
    /// Paused until the user answers.
    Asking(QuestionView),
}

/// Run `turn` to its end, and record and announce how it ended.
pub(super) async fn run(state: AppState, mut turn: Running) {
    let end = match drive(&state, &mut turn).await {
        Ok(end) => end,
        Err(failure) => End::Over("failed", failure),
    };
    state.assistant.stop_requested(turn.turn);
    match settle(&state, &turn, &end).await {
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

/// Record how the turn ended, or that it now waits on a question.
async fn settle(
    state: &AppState,
    turn: &Running,
    end: &End,
) -> Result<(TurnView, i64), super::AssistantError> {
    match end {
        End::Over(ended, words) => {
            turns::finish(&state.pool, turn.user, turn.turn, ended, words).await
        }
        End::Asking(question) => asking::pause(&state.pool, turn.user, turn.turn, question).await,
    }
}

/// The loop. `Err` is a failure, in words for the user.
async fn drive(state: &AppState, turn: &mut Running) -> Result<End, String> {
    let assistant = &state.assistant;
    let mut tools = prompt::tools(&state.tools);
    tools.push(ask::tool());
    let (mut spent, mut balance) = (turn.spent, turn.balance);
    let mut revision = calls::revision(state, turn.user, turn.project).await;
    loop {
        if assistant.stop_requested(turn.turn) {
            return Ok(End::Over("stopped", "Stopped, as you asked.".into()));
        }
        if spent >= assistant.cap_micros() {
            return Ok(End::Over(
                "capped",
                format!(
                    "Stopped: this message has cost {}, the most one message may. Send another to \
                 carry on.",
                    dollars(spent)
                ),
            ));
        }
        if balance <= 0 {
            return Ok(End::Over(
                "capped",
                format!(
                    "Stopped: your balance is {}. Add credit to carry on.",
                    dollars(balance)
                ),
            ));
        }
        let mut messages = turn.history.clone();
        messages.extend(turn.messages.iter().cloned());
        let request = Request {
            model: turn.model,
            effort: EFFORT,
            system: prompt::SYSTEM.to_owned(),
            tools: tools.clone(),
            messages,
        };
        let (reply, latency) = ask(state, turn, request).await.map_err(|error| {
            eprintln!("scorsese-server: assistant: turn {}: {error}", turn.turn);
            format!("The assistant could not be reached: {error}")
        })?;
        let charge = Charge {
            turn: turn.turn,
            project: turn.project,
            prompt: &turn.prompt,
            model: turn.model,
            latency,
        };
        let (charged, now) = turns::charge(&state.pool, turn.user, &charge, &reply)
            .await
            .map_err(|error| error.to_string())?;
        (spent, balance) = (spent + charged, now);
        announce(state, turn, balance).await;
        match &reply.stop {
            Stop::EndTurn => {
                keep(state, turn, reply.native.clone(), reply.message.clone()).await?;
                return Ok(End::Over("answered", reply.text()));
            }
            Stop::ToolUse => {
                keep(state, turn, reply.native.clone(), reply.message.clone()).await?;
                let asked = reply.calls();
                if let Some(question) = ask::alone(&asked) {
                    return Ok(End::Asking(question));
                }
                let mut results = Vec::new();
                for call in asked {
                    results.push(if call.name == ask::NAME {
                        ask::refused(&call)
                    } else {
                        calls::run(state, turn.user, turn.turn, &call).await
                    });
                }
                let now = calls::revision(state, turn.user, turn.project).await;
                if let Some(now) = now.filter(|now| Some(*now) != revision) {
                    let id = turn.project;
                    state
                        .events
                        .send(turn.user, Event::Project { id, revision: now });
                }
                revision = now;
                let results = Message::User { content: results };
                let native = chat::freeze(turn.model, &results).map_err(|e| e.to_string())?;
                keep(state, turn, native, results).await?;
            }
            Stop::Refusal { explanation, .. } => {
                let why = explanation.as_deref().unwrap_or("no reason was given");
                return Ok(End::Over(
                    "refused",
                    format!("{} declined this request: {why}", turn.model.label()),
                ));
            }
            Stop::MaxTokens => {
                return Ok(End::Over(
                    "failed",
                    "The reply ran past its length limit. Ask for \
                                         less at once, or send another message to carry on."
                        .into(),
                ));
            }
            Stop::Other(reason) => {
                return Ok(End::Over(
                    "failed",
                    format!("The reply stopped early ({reason})."),
                ));
            }
        }
    }
}

/// Send `request`, retrying a failure worth retrying while nothing of the
/// reply has reached the browser yet. The reply, and how long the attempt
/// that answered took.
async fn ask(
    state: &AppState,
    turn: &Running,
    request: Request,
) -> Result<(Reply, Duration), ChatError> {
    let request = Arc::new(request);
    let mut waits = RETRY_AFTER.iter();
    loop {
        let (chat, request_) = (turn.chat.clone(), request.clone());
        let mut relay = Relay::new(state.events.clone(), turn.user, turn.turn);
        let sent = Instant::now();
        let (reply, heard) = tokio::task::spawn_blocking(move || {
            let reply = chat.reply(&request_, &mut |piece| relay.hear(piece));
            relay.flush();
            (reply, relay.heard)
        })
        .await
        .map_err(|_| ChatError::Claude(chat::ClaudeError::Cut))?;
        match (reply, waits.next()) {
            (Err(error), Some(wait)) if error.retryable() && !heard => {
                eprintln!(
                    "scorsese-server: assistant: turn {}: {error}; retrying",
                    turn.turn
                );
                tokio::time::sleep(*wait).await;
            }
            (reply, _) => return reply.map(|reply| (reply, sent.elapsed())),
        }
    }
}

/// Add `message` to the turn — its bytes and its record — and keep it.
async fn keep(
    state: &AppState,
    turn: &mut Running,
    native: Box<RawValue>,
    message: Message,
) -> Result<(), String> {
    turn.messages.push(native);
    turn.record.push(message);
    turns::keep(
        &state.pool,
        turn.user,
        turn.turn,
        &turn.messages,
        &turn.record,
    )
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
