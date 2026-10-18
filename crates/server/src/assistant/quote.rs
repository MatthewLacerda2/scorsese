//! The user's answer to a quote the assistant showed them.
//!
//! A quote **pauses the turn** that asked for it (#1440), the way a question
//! does (#710): the call that quoted waits for its result, and the answer is
//! that result, so the same turn carries on. **Yes** spends: the server calls
//! the paid tool itself with the held token, recorded as the user's own call
//! (`client = 'user'`), and the result is what the spend answered. **No**
//! withdraws the token, and the result says they declined. **A change**
//! (#709) is a no that keeps talking: the token is withdrawn, and the result
//! carries their words as a change to the quoted items — so the model
//! rewrites those briefs (`asset_set`) and quotes again, and the new quote
//! needs its own yes. **A message written instead** of answering withdraws it
//! too, and the result carries the message.
//!
//! Every answer takes the token from the paused turn under one lock first, so
//! two clicks cannot answer one quote twice — and none of them shows the
//! model the token: only a yes uses it, and the server makes that call
//! itself. The result is the server's words, so neither a tool's output nor
//! the user can forge one.

use scorsese_providers::chat::{Part, ResultPart};
use serde::Serialize;
use serde_json::json;

use super::AssistantError;
use super::ask::resuming;
use super::store::{QuoteView, TurnView, asking, view};
use super::turn::{self, Resumed};
use crate::credits::ledger;
use crate::db::{self, UserId};
use crate::events::Event;
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
    /// The turn the quote paused, now carrying on with the answer.
    pub turn: Option<TurnView>,
    /// Why no turn carries on, when none does. Always `null` since #1440,
    /// when the paused turn began resuming with every answer.
    pub note: Option<String>,
}

/// How the user answered a quote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// Spend it — or, with `batch`, the half-price batch it offered beside
    /// itself (#947).
    Confirm {
        /// Spend the offered batch instead of the quote.
        batch: bool,
    },
    /// Withdraw it; nothing more is said.
    Decline,
    /// Withdraw it, and ask the assistant for this change to what it covered.
    Change(String),
    /// Withdraw it unanswered: the user wrote this message instead.
    Instead(String),
}

impl Answer {
    /// How the turn records it: as its quote's answer, and as the reply the
    /// panel shows, with the person's words.
    fn recorded(&self) -> (&'static str, (&'static str, Option<&str>)) {
        match self {
            Self::Confirm { .. } => ("confirmed", ("confirmed", None)),
            Self::Decline => ("declined", ("declined", None)),
            // A change spends nothing, and the result carries the rest.
            Self::Change(text) => ("declined", ("changed", Some(text.trim()))),
            Self::Instead(text) => ("withdrawn", ("withdrawn", Some(text.as_str()))),
        }
    }
}

/// Answer the quote `user`'s turn `id` is paused on, and resume that turn
/// with the answer as the result of the call that quoted. The turn carries
/// on on the event stream.
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
    let mut tx = db::scoped(&state.pool, user).await?;
    let paused = asking::paused(&mut tx, id).await?;
    let Some((paused, (token, quote))) =
        paused.and_then(|paused| paused.quote.clone().map(|quote| (paused, quote)))
    else {
        return Err(match view(&mut tx, id).await? {
            Some(_) => AssistantError::Invalid(
                "there is no quote waiting for an answer on that turn".into(),
            ),
            None => AssistantError::NotFound,
        });
    };
    let call = quote
        .call
        .clone()
        .ok_or_else(|| AssistantError::Internal(format!("turn {id}'s quote names no call")))?;
    // No credit check: a no costs nothing, a yes is checked by the paid tool,
    // and the resumed turn stops at once on an empty balance.
    let mut running = resuming(state, &mut tx, user, id, &paused).await?;
    let (recorded, reply) = answer.recorded();
    let view = asking::answered(&mut tx, id, recorded, reply).await?;
    tx.commit().await?;
    state.events.send(
        user,
        Event::ChatTurn {
            turn: view.clone(),
            balance_micros: running.balance,
        },
    );

    let (spent, refused, result) = match answer {
        Answer::Confirm { batch } => {
            let (said, refused) =
                confirm(state, user, (id, paused.project), (&token, batch), &quote).await;
            let result = if refused {
                format!(
                    "The person said yes, but {} refused to spend: {said}",
                    quote.tool
                )
            } else {
                format!(
                    "The person confirmed the quote, and {} answered:\n{said}",
                    quote.tool
                )
            };
            (Some(said), refused, result)
        }
        other => {
            withdraw(state, user, &token).await;
            let result = match other {
                Answer::Change(text) => change_note(&quote, text.trim()),
                Answer::Instead(text) => format!(
                    "The person did not answer the quote; it is withdrawn and nothing was spent. \
                     They wrote instead:\n{text}"
                ),
                _ => "The person declined the quote; nothing was spent.".to_owned(),
            };
            (None, false, result)
        }
    };
    // A yes has just spent: the turn goes on from the balance it left.
    if let Ok(now) = balance(state, user).await {
        running.balance = now;
    }
    running.answer = Some(Resumed::Settle(Part::Result {
        call,
        name: quote.tool.clone(),
        content: vec![ResultPart::Text { text: result }],
        is_error: refused,
    }));
    tokio::spawn(turn::run(state.clone(), running));
    Ok(Answered {
        spent,
        refused,
        turn: Some(view),
        note: None,
    })
}

/// `user`'s balance now.
async fn balance(state: &AppState, user: UserId) -> Result<i64, sqlx::Error> {
    let mut tx = db::scoped(&state.pool, user).await?;
    let balance = ledger::balance(&mut tx).await?;
    tx.commit().await?;
    Ok(balance)
}

/// Withdraw `token`, saying so if it fails: it lapses on its own anyway.
async fn withdraw(state: &AppState, user: UserId, token: &str) {
    if let Err(error) = state.tools.withdraw_quote(user, token).await {
        eprintln!("scorsese-server: assistant: withdrawing a quote: {error}");
    }
}

/// Spend `quote` with `token` as the user, on turn `id` of `project` — or,
/// with `batch`, the half-price batch offered beside it. Whichever of the two
/// was not picked is withdrawn after. What the paid tool said, and whether it
/// refused.
async fn confirm(
    state: &AppState,
    user: UserId,
    (id, project): (i64, i64),
    (token, batch): (&str, bool),
    quote: &QuoteView,
) -> (String, bool) {
    let offer = match state.tools.batch_offer(user, token).await {
        Ok(offer) => offer,
        Err(error) => {
            withdraw(state, user, token).await;
            return (
                format!("the quote could not be read: {error}; nothing was spent"),
                true,
            );
        }
    };
    let (spent, other) = match (batch, offer) {
        (false, offer) => (token.to_owned(), offer),
        (true, Some(offer)) => (offer, Some(token.to_owned())),
        (true, None) => {
            withdraw(state, user, token).await;
            let lapsed = "the half-price batch it offered had lapsed; nothing was spent. Quote \
                          again for a new offer";
            return (lapsed.to_owned(), true);
        }
    };
    let mut arguments = json!({ "project": project, "confirm": spent });
    if batch || quote.batched {
        arguments["batch"] = json!(true);
    }
    let client = Client::User { turn: id };
    let outcome = state
        .tools
        .call(user, client, &quote.tool, &arguments)
        .await;
    // The yes took its own token; the one not picked is forgotten.
    if let Some(other) = other {
        withdraw(state, user, &other).await;
    }
    match outcome {
        Ok(reply) => {
            let words: Vec<String> = reply.parts.into_iter().map(|part| part.text).collect();
            (words.join("\n"), false)
        }
        Err(refusal) => (refusal, true),
    }
}

/// What the model is told when the user asks for a change instead of a yes:
/// the framing is the server's, the words after it the user's own.
fn change_note(quote: &QuoteView, text: &str) -> String {
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
        "The person did not say yes: their answer asks for a change to {covered}, and nothing \
         was spent. Make the change yourself by rewriting those briefs with asset_set — they say \
         what they want, you write the brief — then call {} again without confirm, so they are \
         shown a new quote to answer. Generate nothing and change nothing else meanwhile. Their \
         words:\n{text}",
        quote.tool
    )
}
