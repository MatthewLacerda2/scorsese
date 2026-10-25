//! Starting a turn: check it may run, write it down, and set it going.

use scorsese_providers::chat::{self, Effort, Message};

use super::store::TurnView;
use super::store::turns::{self, Beginning, Last};
use super::store::{asking, hands};
use super::turn::{self, Running};
use super::{AssistantError, ask, compact, effort, model, prompt, quote, wake};
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
    /// How hard the model thinks on it (#769); `None` is the project's level
    /// (#1443).
    pub effort: Option<Effort>,
    /// Started by the server because the work the conversation ordered
    /// landed (#1441), not by a message: `prompt` is ignored, and the turn
    /// shows a line saying what landed instead.
    pub woken: bool,
    /// What the model reads in place of `prompt`, when `prompt` is words the
    /// web app wrote on the person's behalf rather than theirs. The person
    /// still sees `prompt`, in the site's language; the model must not, or it
    /// would answer in the site's language instead of the person's. A turn
    /// that answers a pause (a question or a quote) still hands the pause
    /// `prompt`, not this (#1505).
    pub told: Option<String>,
}

/// Start a turn in `user`'s conversation about `project`: refused when the
/// assistant is not configured, the balance is empty, or a turn is already
/// running there. The turn as it began; the rest arrives on the event stream.
///
/// When the conversation's last turn is paused on a question (#710), the
/// words are its answer instead, and that turn resumes ([`ask::answer`]); on
/// a quote (#1440), they withdraw it and the turn resumes with them
/// ([`quote::answer`]). A new conversation sets either aside.
///
/// Work the conversation ordered that has landed since is told in a note
/// (`wake::told`); a woken turn is refused when there is none.
pub async fn start(
    state: &AppState,
    user: UserId,
    project: i64,
    mut opening: Opening,
) -> Result<TurnView, AssistantError> {
    let mut prompt = opening.prompt.trim().to_owned();
    if prompt.is_empty() && !opening.woken {
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
    let effort = match opening.effort {
        Some(effort) => effort,
        None => effort::of(&mut tx, project).await?,
    };
    let client = state.assistant.chat(model)?;
    let balance = ledger::spendable(&mut tx).await?;
    if balance <= 0 {
        return Err(AssistantError::NoCredit(dollars(balance)));
    }
    let veo = ledger::balance(&mut tx).await? > 0;
    let left = if opening.fresh {
        asking::set_aside_in(&mut tx, project).await?
    } else {
        Vec::new()
    };
    let (session, new) = turns::session(&mut tx, project, opening.fresh).await?;
    let scripted = stored.document.script.is_some();
    let about = prompt::about(project, &stored.summary.name, scripted);
    let mut notes = Vec::new();
    if new {
        notes.push(about.clone());
    }
    if let Some(last) = turns::last(&mut tx, session).await? {
        if last.state == "running" {
            return Err(AssistantError::Busy);
        }
        if last.state == "asking" {
            if opening.woken {
                return Err(AssistantError::Busy);
            }
            drop(tx);
            if last.token.is_some() {
                let instead = quote::Answer::Instead(prompt);
                let answered = quote::answer(state, user, last.id, instead).await?;
                return answered.turn.ok_or(AssistantError::NotFound);
            }
            return ask::answer(state, user, last.id, ask::Answering::words(&prompt)).await;
        }
        notes.extend(settle_quote(state, user, &mut tx, &last).await?);
        if hands::rewrote_script(&mut tx, project, last.id).await? {
            notes.push(prompt::SCRIPT_REWRITTEN.into());
        }
    }
    match wake::told(&mut tx, session, opening.woken).await? {
        Some((note, marker)) => {
            notes.push(note);
            if opening.woken {
                prompt = marker;
            }
        }
        None if opening.woken => {
            return Err(AssistantError::Invalid("no work has landed".into()));
        }
        None => {}
    }
    notes.append(&mut opening.notes);
    let kept = turns::history(&mut tx, session).await?;
    let written = |error: serde_json::Error| AssistantError::Internal(error.to_string());
    let history = chat::replay(model, &kept).map_err(written)?;
    let said: Vec<Message> = kept.iter().flat_map(|turn| turn.record.clone()).collect();
    notes.extend(super::veo::note(&said, veo).map(String::from));
    // A woken turn's user message is the server's, never the person's words:
    // the marker is for the transcript and the ledger. Words the app wrote
    // for them are shown, never read (`told`).
    let words = if opening.woken {
        prompt::WOKEN
    } else {
        opening.told.as_deref().unwrap_or(&prompt)
    };
    let record = prompt::opening(&said, words, &notes, effort);
    // Past its size, the turn opens from a summary instead (#1438).
    let compacting = if compact::due(&mut tx, session).await? {
        Some(compact::prepare(model, &history, &said, &about, &notes, effort).map_err(written)?)
    } else {
        None
    };
    let first = record
        .iter()
        .map(|message| chat::freeze(model, message))
        .collect::<Result<Vec<_>, _>>()
        .map_err(written)?;
    let beginning = Beginning {
        prompt: &prompt,
        model,
        effort,
        woken: opening.woken,
        native: &first,
        record: &record,
    };
    let view = turns::begin(&mut tx, session, &beginning).await?;
    tx.commit().await?;
    // The old conversation's quote, set aside above, is spent by nobody now.
    for token in left {
        if let Err(error) = state.tools.withdraw_quote(user, &token).await {
            eprintln!("scorsese-server: assistant: withdrawing a quote: {error}");
        }
    }

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
        veo,
        spent: 0,
        answer: None,
        compacting,
    };
    tokio::spawn(turn::run(state.clone(), running));
    Ok(view)
}

/// What the model is told about the last turn's quote, when that turn ended
/// without its answer: one held before quotes paused their turn (#1440), which
/// a new message withdraws, since its token must not outlive the box it was
/// shown in — or one withdrawn when the turn was stopped while it waited.
async fn settle_quote(
    state: &AppState,
    user: UserId,
    tx: &mut db::Tx,
    last: &Last,
) -> Result<Option<String>, AssistantError> {
    if let Some(token) = &last.token {
        state.tools.withdraw_quote(user, token).await?;
        turns::withdraw_quote(tx, last.id).await?;
        return Ok(Some(
            "The person did not answer the quote you showed them; it is withdrawn and nothing \
             was spent. Their new message follows."
                .into(),
        ));
    }
    Ok(match (last.state.as_str(), last.answer.as_deref()) {
        ("stopped", Some("withdrawn")) => Some(
            "The person stopped the turn instead of answering the quote you showed them; it is \
             withdrawn and nothing was spent."
                .into(),
        ),
        _ => None,
    })
}
