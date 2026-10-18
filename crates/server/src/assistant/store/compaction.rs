//! What compacting a conversation reads and writes (#1438): how big it was
//! when last sent, and the turn that summarised it.

use scorsese_providers::chat::Message;
use serde_json::value::RawValue;
use sqlx::postgres::PgPool;

use super::turns::{frozen, recorded};
use crate::db::{self, Tx, UserId};

/// How many tokens the latest call to the model in `session` was sent: what
/// it read at full price, from the cache and into it. Every call in a turn
/// re-reads the whole conversation, so the last one's input is its size,
/// counted once. `None` before any call.
pub(in crate::assistant) async fn size(
    tx: &mut Tx,
    session: i64,
) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT c.input_tokens + c.cache_read_tokens
                + c.cache_write_5m_tokens + c.cache_write_1h_tokens
         FROM model_calls c JOIN chat_turns t ON t.id = c.turn_id
         WHERE t.session_id = $1
         ORDER BY c.id DESC LIMIT 1",
    )
    .bind(session)
    .fetch_optional(&mut **tx)
    .await
}

/// Keep `native` and `record` as turn `turn`'s messages, its first carrying
/// the summary of every turn before it, and mark it as where its
/// conversation's history now begins — in one statement, so the history
/// never starts at a turn without its summary.
pub(in crate::assistant) async fn compacted(
    pool: &PgPool,
    user: UserId,
    turn: i64,
    native: &[Box<RawValue>],
    record: &[Message],
) -> Result<(), sqlx::Error> {
    let mut tx = db::scoped(pool, user).await?;
    sqlx::query("UPDATE chat_turns SET messages = $2, record = $3, compacted = true WHERE id = $1")
        .bind(turn)
        .bind(frozen(native))
        .bind(recorded(record))
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}
