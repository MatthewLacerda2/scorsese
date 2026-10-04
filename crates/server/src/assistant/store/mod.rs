//! `chat_sessions`, `chat_turns`, and the turn-linked rows of `tool_calls`:
//! what the assistant said and did, as its owner reads it back.
//!
//! Reads here, writes in [`turns`] — and in `calls`, which records each call
//! to the model as a typed `model_calls` row (#707) and is never read back. Every query runs scoped to the user it is
//! for, so row-level security is the owner filter.

mod calls;
pub(super) mod turns;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::postgres::PgPool;

use super::AssistantError;
use super::model::{self, Choice};
use crate::db::{self, Tx, UserId};
use crate::http::AppState;

/// A turn as its owner sees it: in the conversation, and on the event stream.
#[derive(Debug, Clone, PartialEq, Serialize, sqlx::FromRow)]
pub struct TurnView {
    /// Its id.
    pub id: i64,
    /// The conversation it is part of.
    pub session: i64,
    /// The project that conversation is about.
    pub project: i64,
    /// What the user wrote.
    pub prompt: String,
    /// `running`, `answered`, `refused`, `capped`, `stopped`, `failed` or
    /// `interrupted`.
    pub state: String,
    /// The assistant's final answer, or why there is none.
    pub answer: Option<String>,
    /// The API's reason the last call stopped.
    pub stop_reason: Option<String>,
    /// The model it ran on.
    pub model: String,
    /// How many times the model was called.
    pub calls: i32,
    /// Input tokens read at full price, over the turn.
    pub input_tokens: i64,
    /// Output tokens, thinking included.
    pub output_tokens: i64,
    /// Input written to the prompt cache.
    pub cache_write_tokens: i64,
    /// Input read from the prompt cache.
    pub cache_read_tokens: i64,
    /// What it has cost the user so far, in micro-dollars.
    pub charged_micros: i64,
    /// A paid tool's quote the turn is waiting on — a [`QuoteView`] — or
    /// `null`.
    pub quote: Option<Value>,
    /// How the user answered it: `confirmed`, `declined`, `withdrawn` (they
    /// wrote something else instead), or `null` while it waits.
    pub quote_answer: Option<String>,
    /// When it started, in seconds since the Unix epoch.
    pub started_at: i64,
    /// When it ended.
    pub finished_at: Option<i64>,
}

/// A quote as the confirmation box shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteView {
    /// The paid tool that quoted it, and that a yes calls again.
    pub tool: String,
    /// Each thing the yes would pay for, with what it describes (#709).
    /// Empty on a quote held before there were items.
    #[serde(default)]
    pub items: Vec<QuoteItem>,
    /// The rest of the quote in words: the total, and any line no item
    /// claims. A quote held before #709 has every line here.
    pub lines: Vec<String>,
    /// What it takes from the balance, in micro-dollars.
    pub micros: i64,
    /// When it stops being good, in seconds since the Unix epoch.
    pub expires_at: i64,
}

/// One thing a quote would pay for, as the box shows it: the line the paid
/// tool priced it with, and the description that would be sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteItem {
    /// What it is — an asset id, or `design` for a voice design.
    pub subject: String,
    /// What the paid tool said about it: the price, and how it got there.
    pub says: String,
    /// What `description` is.
    pub brief: BriefKind,
    /// The words that would be sent: a prompt, a line to speak, a voice's
    /// description.
    pub description: String,
}

/// What a quoted item's description is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BriefKind {
    /// A generated shot's or still's prompt.
    Prompt,
    /// A narration line, spoken as written.
    Line,
    /// A designed voice's description.
    Voice,
}

/// A project's current conversation: its newest session's turns, oldest
/// first, and the model the next one runs on.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Conversation {
    /// The project.
    pub project: i64,
    /// The session, or `null` before the first turn.
    pub session: Option<i64>,
    /// Its turns. The last one's `finished_at` is what the switch warning
    /// measures the cache's age from — the server's clock, not the browser's.
    pub turns: Vec<TurnView>,
    /// The id of the model the project runs on.
    pub model: String,
    /// Every model the picker offers.
    pub models: Vec<Choice>,
}

/// One tool call a turn made, as the log records it.
#[derive(Debug, Clone, PartialEq, Serialize, sqlx::FromRow)]
pub struct ToolCallView {
    /// Its row in `tool_calls`.
    pub id: i64,
    /// Its place in the turn, from 1.
    pub position: i32,
    /// `assistant`, or `user` for the user's own yes to a quote.
    pub client: String,
    /// The tool.
    pub tool: String,
    /// What it was called with.
    pub arguments: Value,
    /// `answered`, `refused`, or `null` while it runs.
    pub outcome: Option<String>,
    /// Its words.
    pub reply: Option<String>,
    /// When it started, in seconds since the Unix epoch.
    pub started_at: i64,
    /// When it answered.
    pub finished_at: Option<i64>,
}

/// A turn, and every tool call it made.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TurnDetail {
    /// The turn.
    pub turn: TurnView,
    /// Its calls, in order.
    pub tools: Vec<ToolCallView>,
}

/// The query a [`TurnView`] is read by, ending in `$tail` — a literal, so
/// the whole statement is one static string, as sqlx requires.
macro_rules! turns_where {
    ($tail:literal) => {
        concat!(
            "SELECT t.id, t.session_id AS session, s.project_id AS project, t.prompt,
                    t.state, t.answer, t.stop_reason, t.model, t.calls, t.input_tokens,
                    t.output_tokens, t.cache_write_tokens, t.cache_read_tokens,
                    t.charged_micros, t.quote, t.quote_answer,
                    extract(epoch FROM t.started_at)::bigint AS started_at,
                    extract(epoch FROM t.finished_at)::bigint AS finished_at
             FROM chat_turns t JOIN chat_sessions s ON s.id = t.session_id WHERE ",
            $tail
        )
    };
}

/// Turn `id`, in the caller's transaction.
pub(super) async fn view(tx: &mut Tx, id: i64) -> Result<Option<TurnView>, sqlx::Error> {
    sqlx::query_as(turns_where!("t.id = $1"))
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
}

/// `user`'s turn `id`, on its own.
pub(super) async fn view_of(
    pool: &PgPool,
    user: UserId,
    id: i64,
) -> Result<TurnView, AssistantError> {
    let mut tx = db::scoped(pool, user).await?;
    let turn = view(&mut tx, id).await?;
    tx.commit().await?;
    turn.ok_or(AssistantError::NotFound)
}

/// `user`'s current conversation about `project`.
pub async fn conversation(
    state: &AppState,
    user: UserId,
    project: i64,
) -> Result<Conversation, AssistantError> {
    let mut tx = db::scoped(&state.pool, user).await?;
    let chosen = model::of(&mut tx, project).await?;
    let session = turns::newest_session(&mut tx, project).await?;
    let turns = match session {
        Some(session) => {
            sqlx::query_as(turns_where!("t.session_id = $1 ORDER BY t.id"))
                .bind(session)
                .fetch_all(&mut *tx)
                .await?
        }
        None => Vec::new(),
    };
    tx.commit().await?;
    Ok(Conversation {
        project,
        session,
        turns,
        model: chosen.id().to_owned(),
        models: model::choices(&state.assistant),
    })
}

/// `user`'s turn `id`, with the log of its tool calls.
pub async fn detail(pool: &PgPool, user: UserId, id: i64) -> Result<TurnDetail, AssistantError> {
    let mut tx = db::scoped(pool, user).await?;
    let turn = view(&mut tx, id).await?.ok_or(AssistantError::NotFound)?;
    let tools = sqlx::query_as(
        "SELECT id, position, client, tool, arguments, outcome, reply,
                extract(epoch FROM started_at)::bigint AS started_at,
                extract(epoch FROM finished_at)::bigint AS finished_at
         FROM tool_calls WHERE turn_id = $1 ORDER BY position",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(TurnDetail { turn, tools })
}

/// Mark every turn a dead process left running as `interrupted`: the one
/// cross-user step, run at startup while no turn can be running.
pub async fn recover(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let mut tx = db::privileged(pool).await?;
    let done = sqlx::query(
        "UPDATE chat_turns SET state = 'interrupted', finished_at = now(),
                answer = coalesce(answer, 'The server restarted while this was being worked on.')
         WHERE state = 'running'",
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(done.rows_affected())
}
