//! A turn paused on a question to the user (#710) or on a quote (#1440):
//! pausing it, finding it again, resuming it with the answer, and setting it
//! aside unanswered.
//!
//! Both pauses are the one state, `asking`; which it is, is whether the turn
//! still holds a quote's token.

use sqlx::postgres::PgPool;
use sqlx::types::Json;

use super::{QuestionView, QuoteView, TurnView, view};
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
    let balance = ledger::spendable(&mut tx).await?;
    tx.commit().await?;
    Ok((view, balance))
}

/// Pause turn `turn` on the quote it holds; the turn as it now stands, and
/// the balance.
pub(in crate::assistant) async fn hold(
    pool: &PgPool,
    user: UserId,
    turn: i64,
) -> Result<(TurnView, i64), AssistantError> {
    let mut tx = db::scoped(pool, user).await?;
    sqlx::query("UPDATE chat_turns SET state = 'asking' WHERE id = $1")
        .bind(turn)
        .execute(&mut *tx)
        .await?;
    let view = view(&mut tx, turn).await?.ok_or(AssistantError::NotFound)?;
    let balance = ledger::spendable(&mut tx).await?;
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
    /// The question it waits on — or, on a quote, the last one it asked
    /// before, if any.
    pub(in crate::assistant) question: Option<QuestionView>,
    /// The quote it waits on: its token, and the quote as the box shows it.
    pub(in crate::assistant) quote: Option<(String, QuoteView)>,
}

/// Turn `turn`, locked, if it is paused on a question or a quote.
pub(in crate::assistant) async fn paused(
    tx: &mut Tx,
    turn: i64,
) -> Result<Option<Paused>, sqlx::Error> {
    type Row = (
        i64,
        i64,
        String,
        String,
        String,
        i64,
        Option<Json<QuestionView>>,
        Option<String>,
        Option<Json<QuoteView>>,
    );
    let row: Option<Row> = sqlx::query_as(
        "SELECT t.session_id, s.project_id, t.model, t.effort, t.prompt, t.charged_micros,
                t.questions -> (jsonb_array_length(t.questions) - 1), t.quote_token, t.quote
         FROM chat_turns t JOIN chat_sessions s ON s.id = t.session_id
         WHERE t.id = $1 AND t.state = 'asking' FOR UPDATE OF t",
    )
    .bind(turn)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.map(
        |(session, project, model, effort, prompt, spent, question, token, quote)| Paused {
            session,
            project,
            model,
            effort,
            prompt,
            spent,
            question: question.map(|question| question.0),
            quote: token.zip(quote.map(|quote| quote.0)),
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

/// Set turn `turn` running again, its quote answered `answer` (`confirmed`,
/// `declined` or `withdrawn`) and its token forgotten, with `reply` — the
/// [`QuoteReply`](super::QuoteReply) the panel shows, and the person's
/// `words` if any — added to its replies. The answer reaches the model as the
/// result of the call that quoted, which the running turn keeps.
pub(in crate::assistant) async fn answered(
    tx: &mut Tx,
    turn: i64,
    answer: &str,
    (reply, words): (&str, Option<&str>),
) -> Result<TurnView, AssistantError> {
    let done = sqlx::query(
        "UPDATE chat_turns SET state = 'running', quote_token = NULL, quote_answer = $2,
                quote_replies = quote_replies || jsonb_build_array(jsonb_build_object(
                    'answer', $3::text, 'micros', (quote ->> 'micros')::bigint,
                    'words', $4::text))
         WHERE id = $1 AND state = 'asking'",
    )
    .bind(turn)
    .bind(answer)
    .bind(reply)
    .bind(words)
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

/// End turn `turn` as `stopped` if it is paused on a question or a quote; the
/// turn as it ended, and the token of the quote it held — now the caller's to
/// withdraw — or `None` when it was not paused.
pub(in crate::assistant) async fn set_aside(
    tx: &mut Tx,
    turn: i64,
) -> Result<Option<(TurnView, Option<String>)>, AssistantError> {
    let held: Option<Option<String>> = sqlx::query_scalar(
        "SELECT quote_token FROM chat_turns WHERE id = $1 AND state = 'asking' FOR UPDATE",
    )
    .bind(turn)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(token) = held else {
        return Ok(None);
    };
    sqlx::query(concat!(
        "UPDATE chat_turns SET state = 'stopped', answer = $2, finished_at = now(), ",
        withdrawing!(),
        " WHERE id = $1"
    ))
    .bind(turn)
    .bind(SET_ASIDE)
    .execute(&mut **tx)
    .await?;
    let view = view(tx, turn).await?.ok_or(AssistantError::NotFound)?;
    Ok(Some((view, token)))
}

/// End every turn paused on a question or a quote in `project`'s
/// conversations as `stopped`: a new conversation leaves the old one's
/// question or quote behind. The tokens of the quotes they held, now the
/// caller's to withdraw once this commits.
pub(in crate::assistant) async fn set_aside_in(
    tx: &mut Tx,
    project: i64,
) -> Result<Vec<String>, sqlx::Error> {
    let held: Vec<String> = sqlx::query_scalar(
        "SELECT quote_token FROM chat_turns
         WHERE state = 'asking' AND quote_token IS NOT NULL
           AND session_id IN (SELECT id FROM chat_sessions WHERE project_id = $1)
         FOR UPDATE",
    )
    .bind(project)
    .fetch_all(&mut **tx)
    .await?;
    sqlx::query(concat!(
        "UPDATE chat_turns SET state = 'stopped', answer = $2, finished_at = now(), ",
        withdrawing!(),
        " WHERE state = 'asking'
            AND session_id IN (SELECT id FROM chat_sessions WHERE project_id = $1)"
    ))
    .bind(project)
    .bind(SET_ASIDE)
    .execute(&mut **tx)
    .await?;
    Ok(held)
}
