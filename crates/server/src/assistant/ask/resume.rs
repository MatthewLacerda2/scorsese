//! What resuming a paused turn needs, read under the lock its answer holds:
//! shared by the answer to a question and the answer to a quote (#1440).

use scorsese_providers::chat::{self, Effort, Model, Part, ResultPart};

use crate::assistant::AssistantError;
use crate::assistant::prompt::SCRIPT_REWRITTEN;
use crate::assistant::store::asking::Paused;
use crate::assistant::store::turns::history;
use crate::assistant::turn::Running;
use crate::credits::ledger;
use crate::db::{Tx, UserId};
use crate::http::AppState;

/// `user`'s turn `id`, paused as `paused`, ready to run again from where it
/// stopped — its own messages, the conversation's before them, and the
/// balance now — with nothing to say first yet. Refused when its model has
/// no key; the caller decides what a balance of nothing means.
pub(in crate::assistant) async fn resuming(
    state: &AppState,
    tx: &mut Tx,
    user: UserId,
    id: i64,
    paused: &Paused,
) -> Result<Running, AssistantError> {
    let model = Model::from_id(&paused.model)
        .ok_or_else(|| AssistantError::Internal(format!("unknown model {}", paused.model)))?;
    let effort = Effort::from_name(&paused.effort)
        .ok_or_else(|| AssistantError::Internal(format!("unknown effort {}", paused.effort)))?;
    let client = state.assistant.chat(model)?;
    let balance = ledger::balance(tx).await?;
    // The paused turn is its conversation's last: nothing starts after it
    // while it waits, since a new message is its answer.
    let mut kept = history(tx, paused.session).await?;
    let this = kept
        .pop()
        .ok_or_else(|| AssistantError::Internal(format!("turn {id} has no messages")))?;
    let written = |error: serde_json::Error| AssistantError::Internal(error.to_string());
    let history = chat::replay(model, &kept).map_err(written)?;
    Ok(Running {
        user,
        turn: id,
        project: paused.project,
        prompt: paused.prompt.clone(),
        model,
        effort,
        chat: client,
        history,
        messages: this.native,
        record: this.record,
        balance,
        spent: paused.spent,
        answer: None,
        compacting: None,
    })
}

/// The result a paused turn resumes with, carrying the note that the person
/// rewrote the script while it waited when `told` (#1488): the turn may have
/// read it before it paused.
pub(in crate::assistant) fn noted(result: Part, told: bool) -> Part {
    match result {
        Part::Result {
            call,
            name,
            mut content,
            is_error,
        } if told => {
            content.push(ResultPart::Text {
                text: SCRIPT_REWRITTEN.to_owned(),
            });
            Part::Result {
                call,
                name,
                content,
                is_error,
            }
        }
        other => other,
    }
}
