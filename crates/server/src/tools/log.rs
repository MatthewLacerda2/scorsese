//! `tool_calls`: every call, whoever made it.
//!
//! Written when a call starts and finished when it answers, so a call that
//! takes the server down with it is on record, unfinished. The reply's words
//! are kept; its pictures are not — they are frames of the edit or of the
//! footage, and either can be drawn again.

use scorsese_mcp::Reply;
use serde_json::Value;
use sqlx::postgres::PgPool;

use crate::db::{self, UserId};

/// Who made a call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Client {
    /// The user's own client, over web MCP.
    External,
    /// The built-in assistant (#540).
    Assistant,
}

impl Client {
    /// As the `client` column spells it.
    fn as_str(self) -> &'static str {
        match self {
            Self::External => "external",
            Self::Assistant => "assistant",
        }
    }
}

/// Record the start of a call; its row's id.
pub(super) async fn begin(
    pool: &PgPool,
    user: UserId,
    client: Client,
    tool: &str,
    arguments: &Value,
) -> Result<i64, sqlx::Error> {
    let project = arguments.get("project").and_then(Value::as_i64);
    let mut tx = db::scoped(pool, user).await?;
    let id = sqlx::query_scalar(
        "INSERT INTO tool_calls (user_id, client, tool, project_id, arguments)
         VALUES (member_id(), $1, $2, $3, $4) RETURNING id",
    )
    .bind(client.as_str())
    .bind(tool)
    .bind(project)
    .bind(arguments)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(id)
}

/// Record how call `id` ended. A failure to record is logged and swallowed:
/// the call has already happened, and its answer is still owed.
pub(super) async fn end(pool: &PgPool, user: UserId, id: i64, outcome: &Result<Reply, String>) {
    let (said, reply) = match outcome {
        Ok(reply) => (
            "answered",
            reply
                .parts
                .iter()
                .map(|part| part.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        Err(refusal) => ("refused", refusal.clone()),
    };
    let written = async {
        let mut tx = db::scoped(pool, user).await?;
        sqlx::query(
            "UPDATE tool_calls SET outcome = $2, reply = $3, finished_at = now() WHERE id = $1",
        )
        .bind(id)
        .bind(said)
        .bind(reply)
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    };
    if let Err(error) = written.await {
        eprintln!("scorsese-server: could not record how tool call {id} ended: {error}");
    }
}
