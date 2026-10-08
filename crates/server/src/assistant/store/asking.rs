//! A turn paused on a question to the user (#710): pausing it, finding it
//! again, resuming it with the answer, and setting it aside unanswered.

use sqlx::postgres::PgPool;
use sqlx::types::Json;

use super::{QuestionView, TurnView, view};
use crate::assistant::AssistantError;
use crate::credits::ledger;
use crate::db::{self, Tx, UserId};

/// Pause turn `turn` on `question`; the turn as it now stands, and the
/// balance.
pub(in crate::assistant) async fn pause(
    pool: &PgPool,
    user: UserId,
    turn: i64,
    question: &QuestionView,
) -> Result<(TurnView, i64), AssistantError> {
    let mut tx = db::scoped(pool, user).await?;
    sqlx::query(
        "UPDATE chat_turns SET state = 'asking', questions = questions || jsonb_build_array($2)
         WHERE id = $1",
    )
    .bind(turn)
    .bind(Json(question))
    .execute(&mut *tx)
    .await?;
    let view = view(&mut tx, turn).await?.ok_or(AssistantError::NotFound)?;
    let balance = ledger::balance(&mut tx).await?;
    tx.commit().await?;
    Ok((view, balance))
}

/// What resuming a paused turn needs to know about it.
pub(in crate::assistant) struct Paused {
    /// Its conversation.
    pub(in crate::assistant) session: i64,
    /// That conversation's project.
    pub(in crate::assistant) project: i64,
    /// The model it runs on, as stored.
    pub(in crate::assistant) model: String,
    /// The effort it started at, as stored.
    pub(in crate::assistant) effort: String,
    /// What the user first wrote.
    pub(in crate::assistant) prompt: String,
    /// What it has cost so far, in micro-dollars.
    pub(in crate::assistant) spent: i64,
    /// The question it waits on.
    pub(in crate::assistant) question: QuestionView,
}

/// Turn `turn`, locked, if it is paused on a question.
pub(in crate::assistant) async fn paused(
    tx: &mut Tx,
    turn: i64,
) -> Result<Option<Paused>, sqlx::Error> {
    type Row = (i64, i64, String, String, String, i64, Json<QuestionView>);
    let row: Option<Row> = sqlx::query_as(
        "SELECT t.session_id, s.project_id, t.model, t.effort, t.prompt, t.charged_micros,
                t.questions -> (jsonb_array_length(t.questions) - 1)
         FROM chat_turns t JOIN chat_sessions s ON s.id = t.session_id
         WHERE t.id = $1 AND t.state = 'asking' FOR UPDATE OF t",
    )
    .bind(turn)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.map(
        |(session, project, model, effort, prompt, spent, question)| Paused {
            session,
            project,
            model,
            effort,
            prompt,
            spent,
            question: question.0,
        },
    ))
}

/// Set turn `turn` running again, its last question now `answered` — the
/// same question with the user's answer on it. The answer reaches the model
/// as the turn's next message, which the running turn keeps. Refused as busy
/// if another turn of the conversation is running.
pub(in crate::assistant) async fn resumed(
    tx: &mut Tx,
    turn: i64,
    answered: &QuestionView,
) -> Result<TurnView, AssistantError> {
    let done = sqlx::query(
        "UPDATE chat_turns SET state = 'running',
                questions = jsonb_set(questions,
                    ARRAY[(jsonb_array_length(questions) - 1)::text], $2)
         WHERE id = $1 AND state = 'asking'",
    )
    .bind(turn)
    .bind(Json(answered))
    .execute(&mut **tx)
    .await;
    match done {
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => {
            return Err(AssistantError::Busy);
        }
        done => done?,
    };
    view(tx, turn).await?.ok_or(AssistantError::NotFound)
}

/// What a question set aside unanswered says, as the turn's answer.
const SET_ASIDE: &str = "Stopped while waiting for your answer.";

/// End turn `turn` as `stopped` if it is paused on a question; the turn as it
/// ended, or `None` when it was not paused.
pub(in crate::assistant) async fn set_aside(
    tx: &mut Tx,
    turn: i64,
) -> Result<Option<TurnView>, AssistantError> {
    let done = sqlx::query(
        "UPDATE chat_turns SET state = 'stopped', answer = $2, finished_at = now()
         WHERE id = $1 AND state = 'asking'",
    )
    .bind(turn)
    .bind(SET_ASIDE)
    .execute(&mut **tx)
    .await?;
    if done.rows_affected() == 0 {
        return Ok(None);
    }
    Ok(view(tx, turn).await?)
}

/// End every turn paused on a question in `project`'s conversations as
/// `stopped`: a new conversation leaves the old one's question behind.
pub(in crate::assistant) async fn set_aside_in(
    tx: &mut Tx,
    project: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE chat_turns SET state = 'stopped', answer = $2, finished_at = now()
         WHERE state = 'asking'
           AND session_id IN (SELECT id FROM chat_sessions WHERE project_id = $1)",
    )
    .bind(project)
    .bind(SET_ASIDE)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
