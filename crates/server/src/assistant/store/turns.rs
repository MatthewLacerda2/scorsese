//! Writing a turn: its conversation, its messages as they are added, what
//! each call cost, a quote it holds, and how it ended.

use scorsese_providers::claude::{Response, Settings};
use serde_json::value::RawValue;
use sqlx::postgres::PgPool;

use super::{QuoteView, TurnView, view};
use crate::assistant::AssistantError;
use crate::credits::ledger::{self, AssistantCall};
use crate::db::{self, Tx, UserId};

/// The newest conversation about `project`, if there is one.
pub(in crate::assistant) async fn newest_session(
    tx: &mut Tx,
    project: i64,
) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar("SELECT max(id) FROM chat_sessions WHERE project_id = $1")
        .bind(project)
        .fetch_one(&mut **tx)
        .await
}

/// The conversation a new turn about `project` joins — the newest, or a new
/// one when there is none or `fresh` asks for it — and whether it is new.
pub(in crate::assistant) async fn session(
    tx: &mut Tx,
    project: i64,
    fresh: bool,
) -> Result<(i64, bool), sqlx::Error> {
    if !fresh && let Some(session) = newest_session(tx, project).await? {
        return Ok((session, false));
    }
    let id = sqlx::query_scalar(
        "INSERT INTO chat_sessions (user_id, project_id) VALUES (member_id(), $1) RETURNING id",
    )
    .bind(project)
    .fetch_one(&mut **tx)
    .await?;
    Ok((id, true))
}

/// The last turn of `session`: its id, state, and the quote it still holds.
pub(in crate::assistant) struct Last {
    /// The turn.
    pub(in crate::assistant) id: i64,
    /// Its state.
    pub(in crate::assistant) state: String,
    /// The unspent token of a quote nobody answered.
    pub(in crate::assistant) token: Option<String>,
    /// How its quote was answered, if it had one.
    pub(in crate::assistant) answer: Option<String>,
}

/// `session`'s last turn, if it has one.
pub(in crate::assistant) async fn last(
    tx: &mut Tx,
    session: i64,
) -> Result<Option<Last>, sqlx::Error> {
    let row: Option<(i64, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT id, state, quote_token, quote_answer FROM chat_turns
         WHERE session_id = $1 ORDER BY id DESC LIMIT 1",
    )
    .bind(session)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.map(|(id, state, token, answer)| Last {
        id,
        state,
        token,
        answer,
    }))
}

/// Record how the user answered turn `turn`'s quote, and forget its token.
pub(in crate::assistant) async fn answer_quote(
    tx: &mut Tx,
    turn: i64,
    answer: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE chat_turns SET quote_answer = $2, quote_token = NULL WHERE id = $1")
        .bind(turn)
        .bind(answer)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Every message `session`'s turns have added, in order: the conversation a
/// new turn continues, exactly as it was sent.
pub(in crate::assistant) async fn history(
    tx: &mut Tx,
    session: i64,
) -> Result<Vec<Box<RawValue>>, AssistantError> {
    let turns: Vec<String> =
        sqlx::query_scalar("SELECT messages FROM chat_turns WHERE session_id = $1 ORDER BY id")
            .bind(session)
            .fetch_all(&mut **tx)
            .await?;
    let mut messages = Vec::new();
    for turn in turns {
        let added: Vec<Box<RawValue>> = serde_json::from_str(&turn).map_err(|error| {
            eprintln!("scorsese-server: a stored conversation is unreadable: {error}");
            AssistantError::Invalid("this conversation's record is damaged; start a new one".into())
        })?;
        messages.extend(added);
    }
    Ok(messages)
}

/// Begin a turn in `session`, its first messages `opening`. Refused as busy
/// while another turn of the session runs.
pub(in crate::assistant) async fn begin(
    tx: &mut Tx,
    session: i64,
    prompt: &str,
    settings: &Settings,
    opening: &[Box<RawValue>],
) -> Result<TurnView, AssistantError> {
    let inserted = sqlx::query_scalar(
        "INSERT INTO chat_turns (user_id, session_id, prompt, messages, model, effort)
         VALUES (member_id(), $1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(session)
    .bind(prompt)
    .bind(frozen(opening))
    .bind(&settings.model)
    .bind(settings.effort.as_str())
    .fetch_one(&mut **tx)
    .await;
    let id: i64 = match inserted {
        Ok(id) => id,
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => {
            return Err(AssistantError::Busy);
        }
        Err(error) => return Err(error.into()),
    };
    view(tx, id).await?.ok_or(AssistantError::NotFound)
}

/// Keep `messages` as turn `turn`'s messages so far.
pub(in crate::assistant) async fn keep(
    pool: &PgPool,
    user: UserId,
    turn: i64,
    messages: &[Box<RawValue>],
) -> Result<(), sqlx::Error> {
    let mut tx = db::scoped(pool, user).await?;
    sqlx::query("UPDATE chat_turns SET messages = $2 WHERE id = $1")
        .bind(turn)
        .bind(frozen(messages))
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

/// What one call to the model is charged for, and where.
pub(in crate::assistant) struct Charge<'a> {
    /// The turn.
    pub(in crate::assistant) turn: i64,
    /// Its project.
    pub(in crate::assistant) project: i64,
    /// Its prompt, which the history shows.
    pub(in crate::assistant) prompt: &'a str,
    /// The model the call was made on.
    pub(in crate::assistant) model: &'a str,
}

/// Charge one call's reply and add it to the turn's totals. What it charged,
/// and the balance after.
pub(in crate::assistant) async fn charge(
    pool: &PgPool,
    user: UserId,
    charge: &Charge<'_>,
    reply: &Response,
) -> Result<(i64, i64), AssistantError> {
    let mut tx = db::scoped(pool, user).await?;
    let call = AssistantCall {
        model: charge.model,
        usage: reply.usage,
        project: Some(charge.project),
        prompt: charge.prompt,
        turn: Some(charge.turn),
    };
    let charged = ledger::charge_assistant(&mut tx, &call)
        .await
        .map_err(|error| AssistantError::Internal(error.to_string()))?;
    let usage = reply.usage;
    let count = |tokens: u64| i64::try_from(tokens).unwrap_or(i64::MAX);
    sqlx::query(
        "UPDATE chat_turns SET calls = calls + 1, stop_reason = $2,
                input_tokens = input_tokens + $3, output_tokens = output_tokens + $4,
                cache_write_tokens = cache_write_tokens + $5,
                cache_read_tokens = cache_read_tokens + $6,
                charged_micros = charged_micros + $7
         WHERE id = $1",
    )
    .bind(charge.turn)
    .bind(reply.stop.as_str())
    .bind(count(usage.input))
    .bind(count(usage.output))
    .bind(count(
        usage.cache_write_5m.saturating_add(usage.cache_write_1h),
    ))
    .bind(count(usage.cache_read))
    .bind(charged)
    .execute(&mut *tx)
    .await?;
    let balance = ledger::balance(&mut tx).await?;
    tx.commit().await?;
    Ok((charged, balance))
}

/// Hold a quote on turn `turn` for the user to answer: its token, and what
/// the box shows.
pub(in crate::assistant) async fn hold_quote(
    pool: &PgPool,
    user: UserId,
    turn: i64,
    token: &str,
    quote: &QuoteView,
) -> Result<(), sqlx::Error> {
    let mut tx = db::scoped(pool, user).await?;
    sqlx::query(
        "UPDATE chat_turns SET quote_token = $2, quote = $3, quote_answer = NULL WHERE id = $1",
    )
    .bind(turn)
    .bind(token)
    .bind(sqlx::types::Json(quote))
    .execute(&mut *tx)
    .await?;
    tx.commit().await
}

/// End turn `turn` in `state`, with its answer; the turn as it ended, and the
/// balance.
pub(in crate::assistant) async fn finish(
    pool: &PgPool,
    user: UserId,
    turn: i64,
    state: &str,
    answer: &str,
) -> Result<(TurnView, i64), AssistantError> {
    let mut tx = db::scoped(pool, user).await?;
    sqlx::query("UPDATE chat_turns SET state = $2, answer = $3, finished_at = now() WHERE id = $1")
        .bind(turn)
        .bind(state)
        .bind(answer)
        .execute(&mut *tx)
        .await?;
    let view = view(&mut tx, turn).await?.ok_or(AssistantError::NotFound)?;
    let balance = ledger::balance(&mut tx).await?;
    tx.commit().await?;
    Ok((view, balance))
}

/// Messages as the JSON array stored: each one's text, verbatim.
fn frozen(messages: &[Box<RawValue>]) -> String {
    // A list of raw values serialises as their texts joined; it cannot fail.
    serde_json::to_string(messages).unwrap_or_else(|_| "[]".to_owned())
}
