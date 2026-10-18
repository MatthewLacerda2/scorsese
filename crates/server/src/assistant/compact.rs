//! Compacting a long conversation (#1438): at the start of a new turn, once
//! the conversation has passed [`THRESHOLD`] tokens, the model summarises it
//! and the turn opens from that summary instead, replaying nothing older.
//!
//! Every call in a turn re-reads the whole conversation, and one picked up
//! again after the prompt cache expired re-reads it uncached, so a long one
//! costs more on every call and in the end outgrows the request itself. The
//! shape is the simplest that works: one summary, replacing everything. A
//! verbatim tail kept after it would carry thinking blocks valid only against
//! the transcript the summary replaces, and the summary is plain words, so it
//! survives a change of model the way a turn's record does.
//!
//! The summarising call is a call like any other: the turn's own model, the
//! conversation's tools still declared (so it reads the cached prefix), its
//! `model_calls` row, charged to the turn. Nobody is shown it, and a turn
//! resumed from a pause never compacts: only [`super::start`] prepares one.
//! When it fails, the turn runs on the whole conversation, as it did before.

use std::sync::Arc;
use std::time::{Duration, Instant};

use scorsese_providers::chat::{self, Message, Model, Reply, Request, Stop};
use serde_json::value::RawValue;

use super::store::compaction;
use super::store::turns::{self, Charge};
use super::turn::Running;
use super::{ask, prompt};
use crate::db::Tx;
use crate::http::AppState;

/// The size past which a new turn compacts its conversation, in tokens.
pub(super) const THRESHOLD: i64 = 150_000;

/// What the model is asked to write, as the last message of the summarising
/// call. Its last sentence forbids a tool call: the tools stay declared only
/// so the call reads the cached prefix.
pub(super) const INSTRUCTION: &str = "\
This conversation is about to be compacted: everything above will be replaced \
by a summary you write now, and nothing else of it will be kept. Write that \
summary for yourself, so that you can carry on the edit from it alone. Keep: \
the project and the video being made; the agreed script, or that the \
project's script holds it; every decision and preference the person stated, \
in their own words where the words matter; what is generated, what is still \
a sketch, and which quotes they confirmed or declined; the asset and clip ids \
the edit depends on; open questions, and what you said you would do next. \
Leave out pictures: a frame can be looked at again with still. Write the \
summary as plain text, and do not call any tool.";

/// A compaction a new turn will make before its first call.
pub(super) struct Compacting {
    /// The summarising call's messages: the whole conversation, then
    /// [`INSTRUCTION`].
    request: Vec<Box<RawValue>>,
    /// The server's notes the turn opened with, said again after the summary.
    notes: Vec<String>,
}

/// Whether a new turn in `session` compacts it: past [`THRESHOLD`] when the
/// model was last called.
pub(super) async fn due(tx: &mut Tx, session: i64) -> Result<bool, sqlx::Error> {
    let size = compaction::size(tx, session).await?;
    Ok(size.is_some_and(|size| size > THRESHOLD))
}

/// The compaction of a conversation whose turns were `history` on `model` and
/// said `said`, for a turn opening with `notes`.
pub(super) fn prepare(
    model: Model,
    history: &[Box<RawValue>],
    said: &[Message],
    notes: &[String],
) -> Result<Compacting, serde_json::Error> {
    let mut request = history.to_vec();
    for message in prompt::opening(said, INSTRUCTION, &[]) {
        request.push(chat::freeze(model, &message)?);
    }
    Ok(Compacting {
        request,
        notes: notes.to_vec(),
    })
}

/// Make the compaction `turn` was started with, if any: summarise the
/// conversation, and open the turn again from the summary alone. A failure
/// is logged and leaves the turn as it was.
pub(super) async fn compact(state: &AppState, turn: &mut Running) {
    let Some(compacting) = turn.compacting.take() else {
        return;
    };
    if let Err(error) = summarised(state, turn, compacting).await {
        eprintln!(
            "scorsese-server: assistant: turn {}: not compacted: {error}",
            turn.turn
        );
    }
}

/// [`compact`], its failure in words for the log.
async fn summarised(
    state: &AppState,
    turn: &mut Running,
    compacting: Compacting,
) -> Result<(), String> {
    let mut tools = prompt::tools(&state.tools);
    tools.extend(ask::functions());
    let request = Request {
        model: turn.model,
        effort: turn.effort,
        system: prompt::SYSTEM.to_owned(),
        tools,
        messages: compacting.request,
    };
    let (reply, latency) = call(turn, request).await?;
    let charge = Charge {
        turn: turn.turn,
        project: turn.project,
        prompt: &turn.prompt,
        model: turn.model,
        latency,
    };
    let (charged, balance) = turns::charge(&state.pool, turn.user, &charge, &reply)
        .await
        .map_err(|error| error.to_string())?;
    turn.spent += charged;
    turn.balance = balance;
    let summary = reply.text();
    if !matches!(reply.stop, Stop::EndTurn | Stop::ToolUse) || summary.trim().is_empty() {
        return Err(format!("no summary came back ({})", reply.stop.as_str()));
    }
    let mut notes = vec![note(&summary)];
    notes.extend(compacting.notes);
    let record = prompt::opening(&[], &turn.prompt, &notes);
    let messages = record
        .iter()
        .map(|message| chat::freeze(turn.model, message))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    compaction::compacted(&state.pool, turn.user, turn.turn, &messages, &record)
        .await
        .map_err(|error| error.to_string())?;
    turn.history.clear();
    turn.messages = messages;
    turn.record = record;
    Ok(())
}

/// The server's note carrying `summary`: what the model reads in place of
/// the conversation before the person's message.
fn note(summary: &str) -> String {
    format!(
        "This conversation ran long, so everything before the person's message above was \
         replaced by this summary, which you wrote; nothing older is shown. A picture you \
         saw before can be looked at again with still.\n\n{summary}"
    )
}

/// Send the summarising `request`, streaming nothing to the browser: the
/// reply, and how long it took.
async fn call(turn: &Running, request: Request) -> Result<(Reply, Duration), String> {
    let chat = Arc::clone(&turn.chat);
    let sent = Instant::now();
    let reply = tokio::task::spawn_blocking(move || chat.reply(&request, &mut |_| {}))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    Ok((reply, sent.elapsed()))
}
