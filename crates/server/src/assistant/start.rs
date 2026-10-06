//! Starting a turn: check it may run, write it down, and set it going.

use scorsese_providers::chat::{self, Effort, Message};

use super::store::TurnView;
use super::store::asking;
use super::store::turns::{self, Beginning, Last};
use super::turn::{self, Running};
use super::{AssistantError, DEFAULT_EFFORT, ask, model, prompt};
use crate::credits::{dollars, ledger};
use crate::db::{self, UserId};
use crate::events::Event;
use crate::http::AppState;
use crate::projects::{self, ProjectError};

/// What a turn opens with.
#[derive(Debug, Clone, Default)]
pub struct Opening {
    /// The user's words.
    pub prompt: String,
    /// Begin a new conversation rather than continue the project's newest.
    pub fresh: bool,
    /// What the server has to tell the model besides — said as a `system`
    /// message, which only the server can write.
    pub notes: Vec<String>,
    /// How hard the model thinks on it (#769); `None` is [`DEFAULT_EFFORT`].
    pub effort: Option<Effort>,
}

/// Start a turn in `user`'s conversation about `project`: refused when the
/// assistant is not configured, the balance is empty, or a turn is already
/// running there. The turn as it began; the rest arrives on the event stream.
///
/// When the conversation's last turn is paused on a question (#710), the
/// words are its answer instead, and that turn resumes ([`ask::answer`]); a
/// new conversation sets the question aside.
pub async fn start(
    state: &AppState,
    user: UserId,
    project: i64,
    mut opening: Opening,
) -> Result<TurnView, AssistantError> {
    let prompt = opening.prompt.trim().to_owned();
    let effort = opening.effort.unwrap_or(DEFAULT_EFFORT);
    if prompt.is_empty() {
        return Err(AssistantError::Invalid(
            "say what you would like done".into(),
        ));
    }
    let stored = projects::open(&state.pool, user, project)
        .await
        .map_err(|error| match error {
            ProjectError::NotFound => AssistantError::NotFound,
            other => AssistantError::Internal(other.to_string()),
        })?;

    let mut tx = db::scoped(&state.pool, user).await?;
    let model = model::of(&mut tx, project).await?;
    let client = state.assistant.chat(model)?;
    let balance = ledger::balance(&mut tx).await?;
    if balance <= 0 {
        return Err(AssistantError::NoCredit(dollars(balance)));
    }
    if opening.fresh {
        asking::set_aside_in(&mut tx, project).await?;
    }
    let (session, new) = turns::session(&mut tx, project, opening.fresh).await?;
    let mut notes = Vec::new();
    if new {
        notes.push(prompt::about(project, &stored.summary.name));
    }
    if let Some(last) = turns::last(&mut tx, session).await? {
        if last.state == "running" {
            return Err(AssistantError::Busy);
        }
        if last.state == "asking" {
            drop(tx);
            return ask::answer(state, user, last.id, &prompt).await;
        }
        notes.extend(settle_quote(state, user, &mut tx, &last).await?);
    }
    notes.append(&mut opening.notes);
    let kept = turns::history(&mut tx, session).await?;
    let written = |error: serde_json::Error| AssistantError::Internal(error.to_string());
    let history = chat::replay(model, &kept).map_err(written)?;
    let said: Vec<Message> = kept.iter().flat_map(|turn| turn.record.clone()).collect();
    let record = prompt::opening(&said, &prompt, &notes);
    let first = record
        .iter()
        .map(|message| chat::freeze(model, message))
        .collect::<Result<Vec<_>, _>>()
        .map_err(written)?;
    let beginning = Beginning {
        prompt: &prompt,
        model,
        effort,
        native: &first,
        record: &record,
    };
    let view = turns::begin(&mut tx, session, &beginning).await?;
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
        turn: view.id,
        project,
        prompt,
        model,
        effort,
        chat: client,
        history,
        messages: first,
        record,
        balance,
        spent: 0,
    };
    tokio::spawn(turn::run(state.clone(), running));
    Ok(view)
}

/// What the model is told about the last turn's quote: a no, or a quote
/// nobody answered — which a new message withdraws, since its token must not
/// outlive the box it was shown in.
async fn settle_quote(
    state: &AppState,
    user: UserId,
    tx: &mut db::Tx,
    last: &Last,
) -> Result<Option<String>, AssistantError> {
    if let Some(token) = &last.token {
        state.tools.withdraw_quote(user, token).await?;
        turns::answer_quote(tx, last.id, "withdrawn").await?;
        return Ok(Some(
            "The person did not answer the quote you showed them; it is withdrawn and nothing \
             was spent. Their new message follows."
                .into(),
        ));
    }
    Ok(match last.answer.as_deref() {
        Some("declined") => {
            Some("The person declined the quote you showed them; nothing was spent.".into())
        }
        _ => None,
    })
}
