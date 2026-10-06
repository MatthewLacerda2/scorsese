//! The user's answer to a quote the assistant showed them.
//!
//! **Yes** spends: the server calls the paid tool itself with the held token,
//! recorded as the user's own call (`client = 'user'`), and starts a turn
//! that tells the model — as a `system` message, which it can trust — what the
//! spend did. **No** withdraws the token, and the next turn is told. **A
//! change** (#709) is a no that keeps talking: the token is withdrawn, and a
//! turn starts at once with the user's words as their message and a server
//! note saying they are about the quoted items — so the model rewrites those
//! briefs (`asset_set`) and quotes again, and the new quote needs its own yes.
//! One call does both, so a failed second request can never leave a quote
//! declined and the change unsaid.
//!
//! Every answer claims the token from the turn in one statement first, so two
//! clicks cannot answer one quote twice — and none of them shows the model the
//! token: only a yes uses it, and the server makes that call itself.

use scorsese_providers::chat::Effort;
use serde::Serialize;
use serde_json::json;

use super::AssistantError;
use super::start::{Opening, start};
use super::store::{QuoteView, TurnView};
use crate::db::{self, UserId};
use crate::http::AppState;
use crate::tools::Client;

/// What answering a quote did.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Answered {
    /// What the paid tool said when it spent — `null` for a no.
    pub spent: Option<String>,
    /// Whether the paid tool refused (an expired quote, a brief changed since,
    /// a balance that no longer covers it).
    pub refused: bool,
    /// The turn started to carry on after a yes — `null` for a no, or when no
    /// turn could start (the balance ran out on the spend itself).
    pub turn: Option<TurnView>,
    /// Why no turn started, when one did not.
    pub note: Option<String>,
}

/// How the user answered a quote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// Spend it.
    Confirm,
    /// Withdraw it; nothing more is said.
    Decline,
    /// Withdraw it, and ask the assistant for this change to what it covered.
    Change(String),
}

/// Answer the quote held on `user`'s turn `id`.
pub async fn answer(
    state: &AppState,
    user: UserId,
    id: i64,
    answer: Answer,
) -> Result<Answered, AssistantError> {
    if matches!(&answer, Answer::Change(text) if text.trim().is_empty()) {
        return Err(AssistantError::Invalid(
            "say what you would like changed".into(),
        ));
    }
    let Claimed {
        token,
        quote,
        project,
        effort,
    } = claim(state, user, id, &answer).await?;
    let change = match answer {
        Answer::Confirm => {
            let turn = (id, project, effort);
            return confirm(state, user, turn, &token, &quote).await;
        }
        Answer::Decline => None,
        Answer::Change(text) => Some(text),
    };
    state.tools.withdraw_quote(user, &token).await?;
    let Some(text) = change else {
        return Ok(Answered {
            spent: None,
            refused: false,
            turn: None,
            note: None,
        });
    };
    let opening = Opening {
        prompt: text,
        fresh: false,
        notes: vec![change_note(&quote)],
        effort: Some(effort),
    };
    let (turn, note) = started(start(state, user, project, opening).await);
    Ok(Answered {
        spent: None,
        refused: false,
        turn,
        note,
    })
}

/// Take the quote's token from turn `id`, recording `answer`: the token, the
/// quote as the box showed it, and the project. Refused while a turn of the
/// conversation runs, and when no quote waits on that turn.
async fn claim(
    state: &AppState,
    user: UserId,
    id: i64,
    answer: &Answer,
) -> Result<Claimed, AssistantError> {
    let mut tx = db::scoped(&state.pool, user).await?;
    // The conversation's newest state other than an ended one: a running
    // turn, or one paused on a question (#710), which is answered first.
    let busy: Option<Option<String>> = sqlx::query_scalar(
        "SELECT (SELECT max(r.state) FROM chat_turns r
                 WHERE r.session_id = t.session_id AND r.state IN ('running', 'asking'))
         FROM chat_turns t WHERE t.id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    match busy.as_ref().map(Option::as_deref) {
        None => return Err(AssistantError::NotFound),
        Some(Some("running")) => return Err(AssistantError::Busy),
        Some(Some(_)) => {
            return Err(AssistantError::Invalid(
                "the assistant asked you a question; answer it first".into(),
            ));
        }
        Some(None) => {}
    }
    // A change is recorded as a no: it spends nothing, and the turn it starts
    // carries the rest.
    let recorded = match answer {
        Answer::Confirm => "confirmed",
        Answer::Decline | Answer::Change(_) => "declined",
    };
    let claimed: Option<(String, sqlx::types::Json<QuoteView>, i64, String)> = sqlx::query_as(
        "WITH held AS (
             SELECT id, quote_token FROM chat_turns
             WHERE id = $1 AND quote_token IS NOT NULL FOR UPDATE)
         UPDATE chat_turns t SET quote_token = NULL, quote_answer = $2
         FROM held, chat_sessions s
         WHERE t.id = held.id AND s.id = t.session_id
         RETURNING held.quote_token, t.quote, s.project_id, t.effort",
    )
    .bind(id)
    .bind(recorded)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    let (token, quote, project, effort) = claimed.ok_or_else(|| {
        AssistantError::Invalid("there is no quote waiting for an answer on that turn".into())
    })?;
    let effort = Effort::from_name(&effort)
        .ok_or_else(|| AssistantError::Internal(format!("unknown effort {effort}")))?;
    Ok(Claimed {
        token,
        quote: quote.0,
        project,
        effort,
    })
}

/// A quote taken from its turn to be answered.
struct Claimed {
    /// The token that spends it.
    token: String,
    /// The quote as the box showed it.
    quote: QuoteView,
    /// The project its conversation is about.
    project: i64,
    /// The effort its turn ran at, which the turn the answer starts keeps.
    effort: Effort,
}

/// Spend `quote` with `token` as the user, on turn `id` of `project`, and
/// start the turn that tells the model what the spend did, at `effort`.
async fn confirm(
    state: &AppState,
    user: UserId,
    (id, project, effort): (i64, i64, Effort),
    token: &str,
    quote: &QuoteView,
) -> Result<Answered, AssistantError> {
    let arguments = json!({ "project": project, "confirm": token });
    let client = Client::User { turn: id };
    let outcome = state
        .tools
        .call(user, client, &quote.tool, &arguments)
        .await;
    let (said, refused) = match outcome {
        Ok(reply) => {
            let words: Vec<String> = reply.parts.into_iter().map(|part| part.text).collect();
            (words.join("\n"), false)
        }
        Err(refusal) => (refusal, true),
    };
    let note = if refused {
        format!(
            "The person said yes to the quote, but {} refused to spend: {said}",
            quote.tool
        )
    } else {
        format!(
            "The person confirmed the quote. {} was called with it and answered:\n{said}",
            quote.tool
        )
    };
    let opening = Opening {
        prompt: "Yes, go ahead.".into(),
        fresh: false,
        notes: vec![note],
        effort: Some(effort),
    };
    let (turn, note) = started(start(state, user, project, opening).await);
    Ok(Answered {
        spent: Some(said),
        refused,
        turn,
        note,
    })
}

/// A turn an answer started, or why none did.
fn started(started: Result<TurnView, AssistantError>) -> (Option<TurnView>, Option<String>) {
    match started {
        Ok(turn) => (Some(turn), None),
        Err(error) => (None, Some(error.to_string())),
    }
}

/// What the model is told beside a change the user asked for: the framing is
/// the server's, the words are the user's own message.
fn change_note(quote: &QuoteView) -> String {
    let subjects: Vec<&str> = quote
        .items
        .iter()
        .map(|item| item.subject.as_str())
        .collect();
    let covered = if subjects.is_empty() {
        String::from("what that quote covered")
    } else {
        format!("what that quote covered ({})", subjects.join(", "))
    };
    format!(
        "Their message is not a plain no: it asks for a change to {covered}. Make the change \
         yourself by rewriting those briefs with asset_set — they say what they want, you write \
         the brief — then call {} again without confirm, so they are shown a new quote to \
         answer. Generate nothing and change nothing else meanwhile.",
        quote.tool
    )
}
