//! The user's answer to a question a turn paused on: the turn resumes, the
//! answer the result of the call that asked. And Stop on a paused turn, which
//! sets the question aside unanswered.

use scorsese_providers::chat::{self, Message, Model};

use super::{NAME, answered};
use crate::assistant::AssistantError;
use crate::assistant::store::turns::history;
use crate::assistant::store::{TurnView, asking, view};
use crate::assistant::turn::{self, Running};
use crate::credits::{dollars, ledger};
use crate::db::{self, UserId};
use crate::events::Event;
use crate::http::AppState;

/// Answer the question `user`'s turn `id` is paused on with `text` — an
/// option's words or their own — and resume that turn. The turn as it
/// resumed; the rest arrives on the event stream.
///
/// Refused like a new turn would be — no key for its model, no credit — and
/// then the question still waits.
pub async fn answer(
    state: &AppState,
    user: UserId,
    id: i64,
    text: &str,
) -> Result<TurnView, AssistantError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(AssistantError::Invalid("say what your answer is".into()));
    }
    let mut tx = db::scoped(&state.pool, user).await?;
    let Some(paused) = asking::paused(&mut tx, id).await? else {
        return Err(match view(&mut tx, id).await? {
            Some(_) => AssistantError::Invalid(
                "there is no question waiting for an answer on that turn".into(),
            ),
            None => AssistantError::NotFound,
        });
    };
    let model = Model::from_id(&paused.model)
        .ok_or_else(|| AssistantError::Internal(format!("unknown model {}", paused.model)))?;
    let client = state.assistant.chat(model)?;
    let balance = ledger::balance(&mut tx).await?;
    if balance <= 0 {
        return Err(AssistantError::NoCredit(dollars(balance)));
    }
    // The paused turn is its conversation's last: nothing starts after it
    // while it waits, since a new message is its answer.
    let mut kept = history(&mut tx, paused.session).await?;
    let this = kept
        .pop()
        .ok_or_else(|| AssistantError::Internal(format!("turn {id} has no messages")))?;
    let asked = this.record.last().map(Message::calls).unwrap_or_default();
    let call = asked
        .into_iter()
        .find(|call| call.name == NAME)
        .ok_or_else(|| AssistantError::Internal(format!("turn {id} asked nothing")))?;
    let written = |error: serde_json::Error| AssistantError::Internal(error.to_string());
    let history = chat::replay(model, &kept).map_err(written)?;
    let result = Message::User {
        content: vec![answered(&call, text)],
    };
    let (mut messages, mut record) = (this.native, this.record);
    messages.push(chat::freeze(model, &result).map_err(written)?);
    record.push(result);
    let view = asking::resumed(&mut tx, id, (&messages, &record), text).await?;
    tx.commit().await?;

    state.events.send(
        user,
        Event::ChatTurn {
            turn: view.clone(),
            balance_micros: balance,
        },
    );
    let running = Running {
        user,
        turn: id,
        project: paused.project,
        prompt: paused.prompt,
        model,
        chat: client,
        history,
        messages,
        record,
        balance,
        spent: paused.spent,
    };
    tokio::spawn(turn::run(state.clone(), running));
    Ok(view)
}

/// End `user`'s turn `id` as stopped if it is paused on a question, and say
/// so on the event stream; whether it was.
pub async fn set_aside(state: &AppState, user: UserId, id: i64) -> Result<bool, AssistantError> {
    let mut tx = db::scoped(&state.pool, user).await?;
    let ended = asking::set_aside(&mut tx, id).await?;
    let balance = ledger::balance(&mut tx).await?;
    tx.commit().await?;
    let Some(turn) = ended else {
        return Ok(false);
    };
    state.events.send(
        user,
        Event::ChatTurn {
            turn,
            balance_micros: balance,
        },
    );
    Ok(true)
}
