//! The user's answer to a question a turn paused on: the turn resumes, the
//! answer the result of the call that asked — after anything picked from a
//! picker (#901) is imported. And Stop on a paused turn, which sets the
//! question aside unanswered.

use scorsese_providers::chat::Message;

use super::resume::resuming;
use super::{answered, is_ours, pick};
use crate::assistant::AssistantError;
use crate::assistant::store::{QuestionView, TurnView, asking, view};
use crate::assistant::turn::{self, Resumed};
use crate::credits::{dollars, ledger};
use crate::db::{self, UserId};
use crate::events::Event;
use crate::http::AppState;

/// How the user answered a question or a picker.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Answering {
    /// Their words: an option's, or their own. Beside a pick they are a note
    /// on it, and may be empty.
    pub words: String,
    /// The `key`s of the candidates they picked from a picker — empty for
    /// "none of these" — or `None` when they answered in words alone.
    pub picked: Option<Vec<String>>,
}

impl Answering {
    /// An answer in words alone — a typed message, or a question's option.
    pub fn words(words: &str) -> Self {
        Self {
            words: words.to_owned(),
            picked: None,
        }
    }
}

/// Answer the question `user`'s turn `id` is paused on, and resume that turn.
/// The turn as it resumed; the rest arrives on the event stream.
///
/// Refused like a new turn would be — no key for its model, no credit — and
/// then the question still waits. A pick is refused on a question in words,
/// and when it names a candidate the picker did not offer.
pub async fn answer(
    state: &AppState,
    user: UserId,
    id: i64,
    answering: Answering,
) -> Result<TurnView, AssistantError> {
    let words = answering.words.trim();
    let words = (!words.is_empty()).then(|| words.to_owned());
    if words.is_none() && answering.picked.is_none() {
        return Err(AssistantError::Invalid("say what your answer is".into()));
    }
    let mut tx = db::scoped(&state.pool, user).await?;
    let paused = asking::paused(&mut tx, id).await?;
    let Some((paused, question)) = paused.and_then(|paused| {
        let question = paused.question.clone().filter(|_| paused.quote.is_none())?;
        Some((paused, question))
    }) else {
        return Err(match view(&mut tx, id).await? {
            Some(_) => AssistantError::Invalid(
                "there is no question waiting for an answer on that turn".into(),
            ),
            None => AssistantError::NotFound,
        });
    };
    let picked = match (&answering.picked, question.is_picker()) {
        (None, _) => None,
        (Some(_), false) => {
            return Err(AssistantError::Invalid(
                "that question has nothing to pick; answer it in words".into(),
            ));
        }
        (Some(keys), true) => Some(pick::chosen(&question, keys).map_err(AssistantError::Invalid)?),
    };
    let mut running = resuming(state, &mut tx, user, id, &paused).await?;
    let balance = running.balance;
    if balance <= 0 {
        return Err(AssistantError::NoCredit(dollars(balance)));
    }
    let asked = running
        .record
        .last()
        .map(Message::calls)
        .unwrap_or_default();
    let call = asked
        .into_iter()
        .find(|call| is_ours(&call.name))
        .ok_or_else(|| AssistantError::Internal(format!("turn {id} asked nothing")))?;
    let keys = picked.as_ref().map(|picked| {
        let keys = picked.iter().map(|one| pick::key(*one));
        keys.collect::<Vec<_>>()
    });
    let settled = QuestionView {
        answer: words.clone(),
        picked: keys,
        ..question.clone()
    };
    let view = asking::resumed(&mut tx, id, &settled).await?;
    tx.commit().await?;

    state.events.send(
        user,
        Event::ChatTurn {
            turn: view.clone(),
            balance_micros: balance,
        },
    );
    let state = state.clone();
    tokio::spawn(async move {
        // Words alone answer a question at once; a picker's result waits for
        // what was picked to be brought in, while the turn already shows as
        // running.
        let result = if question.is_picker() {
            let who = (user, id, paused.project);
            let picked = picked.as_deref();
            pick::result(&state, who, &call, picked, words.as_deref()).await
        } else {
            answered(&call, words.as_deref().unwrap_or_default())
        };
        running.answer = Some(Resumed::Next(Message::User {
            content: vec![result],
        }));
        turn::run(state, running).await;
    });
    Ok(view)
}

/// End `user`'s turn `id` as stopped if it is paused on a question or a
/// quote — withdrawing the quote — and say so on the event stream; whether it
/// was.
pub async fn set_aside(state: &AppState, user: UserId, id: i64) -> Result<bool, AssistantError> {
    let mut tx = db::scoped(&state.pool, user).await?;
    let ended = asking::set_aside(&mut tx, id).await?;
    let balance = ledger::balance(&mut tx).await?;
    tx.commit().await?;
    let Some((turn, token)) = ended else {
        return Ok(false);
    };
    if let Some(token) = token {
        state.tools.withdraw_quote(user, &token).await?;
    }
    state.events.send(
        user,
        Event::ChatTurn {
            turn,
            balance_micros: balance,
        },
    );
    Ok(true)
}
