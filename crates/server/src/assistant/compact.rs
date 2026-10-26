//! Compacting a long conversation (#1438): at the start of a new turn, once
//! the conversation has passed [`THRESHOLD`] tokens — or would replay more
//! than [`PICTURES`] pictures (#1562) — the model summarises it and the turn
//! opens from that summary instead, replaying nothing older.
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
//! A summary is only one the model finished (`Stop::EndTurn`): a call that
//! stops on a tool, or fails past its retries, leaves the turn running on the
//! whole conversation, as it did before, and says so only in the log.

use std::sync::Arc;
use std::time::{Duration, Instant};

use scorsese_providers::chat::{
    self, Effort, Message, Model, Part, Reply, Request, ResultPart, Stop,
};
use serde_json::value::RawValue;

use super::store::compaction;
use super::store::turns::{self, Charge};
use super::turn::{RETRY_AFTER, Running};
use super::{ask, prompt};
use crate::db::Tx;
use crate::http::AppState;

/// The size past which a new turn compacts its conversation, in tokens.
pub(super) const THRESHOLD: i64 = 150_000;

/// The most pictures a conversation replays before a new turn compacts it,
/// however few tokens they are: under the 100 images one request may carry
/// on a model with a 200k-token context window (Anthropic's *Vision* page,
/// read 2026-10-25, which counts every image block, an earlier turn's and a
/// tool result's included), with room left for the turn's own.
pub(super) const PICTURES: usize = 64;

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
    /// The server's notes the compacted turn opens with: the project's,
    /// then the turn's own.
    notes: Vec<String>,
}

/// Whether a new turn in `session` compacts it: past [`THRESHOLD`] when the
/// model was last called.
pub(super) async fn due(tx: &mut Tx, session: i64) -> Result<bool, sqlx::Error> {
    let size = compaction::size(tx, session).await?;
    Ok(size.is_some_and(|size| size > THRESHOLD))
}

/// Whether a turn opening with `shown` after a conversation that said `said`
/// would replay more than [`PICTURES`] pictures.
pub(super) fn crowded(said: &[Message], shown: &[Part]) -> bool {
    let parts = said.iter().flat_map(Message::parts).chain(shown);
    let pictures: usize = parts
        .map(|part| match part {
            Part::Image { .. } => 1,
            Part::Result { content, .. } => content
                .iter()
                .filter(|piece| matches!(piece, ResultPart::Image { .. }))
                .count(),
            Part::Text { .. } | Part::Call { .. } => 0,
        })
        .sum();
    pictures > PICTURES
}

/// The compaction of a conversation whose turns were `history` on `model` and
/// said `said`, for a turn opening with `notes`. `about` is the server's note
/// on the project ([`prompt::about`]), said again after the summary whether
/// or not the turn carried it: which project this is stays the server's word,
/// never only the summary's.
pub(super) fn prepare(
    model: Model,
    history: &[Box<RawValue>],
    said: &[Message],
    about: &str,
    notes: &[String],
    effort: Effort,
) -> Result<Compacting, serde_json::Error> {
    let mut request = history.to_vec();
    for message in prompt::opening(said, INSTRUCTION, &[], &[], effort) {
        request.push(chat::freeze(model, &message)?);
    }
    let rest = notes.iter().filter(|note| note.as_str() != about).cloned();
    let kept: Vec<String> = std::iter::once(about.to_owned()).chain(rest).collect();
    Ok(Compacting {
        request,
        notes: kept,
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
        tracing::error!("assistant: turn {}: not compacted: {error}", turn.turn);
    }
}

/// [`compact`], its failure in words for the log.
async fn summarised(
    state: &AppState,
    turn: &mut Running,
    compacting: Compacting,
) -> Result<(), String> {
    let mut tools = prompt::tools(&state.tools, turn.veo);
    tools.extend(ask::functions());
    let request = Request {
        model: turn.model,
        system: prompt::SYSTEM.to_owned(),
        tools,
        messages: compacting.request,
    };
    let summary = summary(state, turn, request).await?;
    let mut notes = compacting.notes;
    notes.insert(1, note(&summary));
    let (words, shown) = opened_with(&turn.record).ok_or("the turn opened without words")?;
    let record = prompt::opening(&[], words, &shown, &notes, turn.effort);
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

/// The words a turn's `record` opened with — its user message's first text,
/// after any results it carried — and what it showed after them: the
/// documents and pictures attached (#1562). Not `Running::prompt`, which for
/// a woken turn is the transcript's marker, never sent as the person's words
/// (#1489).
fn opened_with(record: &[Message]) -> Option<(&str, Vec<Part>)> {
    let content = record.iter().find_map(|message| match message {
        Message::User { content } => Some(content),
        _ => None,
    })?;
    let mut parts = content
        .iter()
        .skip_while(|part| matches!(part, Part::Result { .. }));
    let Some(Part::Text { text }) = parts.next() else {
        return None;
    };
    Some((text.as_str(), parts.cloned().collect()))
}

/// Ask for the summary, charging each reply to the turn: its text, once the
/// model has finished one. A reply that stopped anywhere else (on a tool,
/// at its length) is not a summary, and is asked for once more.
async fn summary(state: &AppState, turn: &mut Running, request: Request) -> Result<String, String> {
    let request = Arc::new(request);
    let mut stopped = String::new();
    for _ in 0..2 {
        let (reply, latency) = call(turn, &request).await?;
        let charge = Charge {
            turn: turn.turn,
            project: turn.project,
            prompt: &turn.prompt,
            model: turn.model,
            latency,
            summary: true,
        };
        let (charged, balance) = turns::charge(&state.pool, turn.user, &charge, &reply)
            .await
            .map_err(|error| error.to_string())?;
        turn.spent += charged;
        turn.balance = balance;
        let text = reply.text();
        if matches!(reply.stop, Stop::EndTurn) && !text.trim().is_empty() {
            return Ok(text);
        }
        stopped = reply.stop.as_str().to_owned();
    }
    Err(format!("no summary came back (it stopped on {stopped})"))
}

/// The server's note carrying `summary`: what the model reads in place of
/// the conversation before the person's message. The summary is the model's
/// own account, and the note says so: the server vouches only for what its
/// other notes say.
fn note(summary: &str) -> String {
    format!(
        "This conversation ran long, so everything before the person's message above was \
         replaced by a summary you wrote of it; nothing older is shown. It is your own \
         account, not something the server checked. A picture you saw before can be looked \
         at again with still. Your summary:\n\n{summary}"
    )
}

/// Send the summarising `request`, streaming nothing to the browser and
/// retrying a failure worth retrying as a turn's own calls do: the reply,
/// and how long the attempt that answered took.
async fn call(turn: &Running, request: &Arc<Request>) -> Result<(Reply, Duration), String> {
    let mut waits = RETRY_AFTER.iter();
    loop {
        let (chat, request_) = (Arc::clone(&turn.chat), Arc::clone(request));
        let sent = Instant::now();
        let reply = tokio::task::spawn_blocking(move || chat.reply(&request_, &mut |_| {}))
            .await
            .map_err(|error| error.to_string())?;
        match (reply, waits.next()) {
            (Err(error), Some(wait)) if error.retryable() => {
                tracing::warn!(
                    "assistant: turn {}: summarising: {error}; retrying",
                    turn.turn
                );
                tokio::time::sleep(*wait).await;
            }
            (reply, _) => {
                return reply
                    .map(|reply| (reply, sent.elapsed()))
                    .map_err(|error| error.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pictures_in_tool_results_count_towards_a_crowded_conversation() {
        let image = || ResultPart::Image {
            media_type: "image/jpeg".into(),
            data: "AA==".into(),
        };
        let still = |n| Message::User {
            content: vec![Part::Result {
                call: format!("toolu_{n}"),
                name: "still".into(),
                content: vec![
                    ResultPart::Text {
                        text: "a frame".into(),
                    },
                    image(),
                ],
                is_error: false,
            }],
        };
        let said: Vec<Message> = (0..PICTURES - 1).map(still).collect();
        let attached = Part::Image {
            media_type: "image/jpeg".into(),
            data: "AA==".into(),
        };
        assert!(
            !crowded(&said, std::slice::from_ref(&attached)),
            "at the limit"
        );
        assert!(crowded(&said, &[attached.clone(), attached]), "past it");
    }

    #[test]
    fn a_compacted_turn_opens_with_its_words_and_what_was_attached() {
        let picture = Part::Image {
            media_type: "image/jpeg".into(),
            data: "AA==".into(),
        };
        let record = [
            Message::assistant("(That turn ended before I could answer.)"),
            Message::User {
                content: vec![
                    Part::Result {
                        call: "toolu_1".into(),
                        name: "still".into(),
                        content: vec![ResultPart::Text { text: "no".into() }],
                        is_error: true,
                    },
                    Part::Text {
                        text: "use these".into(),
                    },
                    Part::Text {
                        text: "Attached document 3 “a.md”:\n\nhi".into(),
                    },
                    picture.clone(),
                ],
            },
            Message::System {
                text: "note".into(),
            },
        ];
        let (words, shown) = opened_with(&record).expect("it opened with words");
        assert_eq!(words, "use these");
        assert_eq!(shown.len(), 2);
        assert_eq!(shown[1], picture);
    }
}
