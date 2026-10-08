//! `tool_calls`: every call, whoever made it.
//!
//! Written when a call starts and finished when it answers, so a call that
//! takes the server down with it is on record, unfinished. The reply's words
//! are kept; its pictures are not — they are frames of the edit or of the
//! footage, and either can be drawn again; what both weigh in tokens is
//! kept as an estimate (`size`, #707). An external call carries the API token
//! it came in on, and the client that token's last MCP `initialize` named.

use scorsese_mcp::Reply;
use serde_json::Value;
use sqlx::postgres::PgPool;

use super::size;
use crate::db::{self, UserId};

/// Who made a call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Client {
    /// The user's own client, over web MCP.
    External {
        /// The API token it came in on, whose `clientInfo` the call is
        /// recorded with — `None` for a caller with no connection, such as
        /// the server's own tests calling the toolbox directly.
        token: Option<i64>,
    },
    /// The built-in assistant (#540), in the chat turn `turn`.
    Assistant {
        /// The turn's row in `chat_turns`.
        turn: i64,
    },
    /// The user themselves, from the web app: their yes to a quote the
    /// assistant showed them in turn `turn`, or the stock they picked from
    /// its picker (#901) — calls the model cannot make.
    User {
        /// The turn whose quote it confirms.
        turn: i64,
    },
    /// The user's own hands in the web editor (#545): a clip dragged, trimmed
    /// or set in the inspector, a file dropped onto a track, a frame looked at.
    Editor,
}

impl Client {
    /// As the `client` column spells it.
    fn as_str(self) -> &'static str {
        match self {
            Self::External { .. } => "external",
            Self::Assistant { .. } => "assistant",
            Self::User { .. } => "user",
            Self::Editor => "editor",
        }
    }

    /// The API token the call came in on, if any.
    fn token(self) -> Option<i64> {
        match self {
            Self::External { token } => token,
            Self::Assistant { .. } | Self::User { .. } | Self::Editor => None,
        }
    }

    /// The chat turn the call belongs to, if any.
    fn turn(self) -> Option<i64> {
        match self {
            Self::External { .. } | Self::Editor => None,
            Self::Assistant { turn } | Self::User { turn } => Some(turn),
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
    // A turn's calls are made one after another, so counting them is their
    // order.
    let id = sqlx::query_scalar(
        "INSERT INTO tool_calls (user_id, client, tool, project_id, arguments, turn_id, position,
                                 api_token_id, client_name, client_version)
         SELECT member_id(), $1, $2, $3, $4, $5,
                CASE WHEN $5::bigint IS NULL THEN NULL
                     ELSE (SELECT count(*) + 1 FROM tool_calls WHERE turn_id = $5)::int END,
                $6, t.client_name, t.client_version
         FROM (SELECT 1) one LEFT JOIN api_tokens t ON t.id = $6
         RETURNING id",
    )
    .bind(client.as_str())
    .bind(tool)
    .bind(project)
    .bind(arguments)
    .bind(client.turn())
    .bind(client.token())
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(id)
}

/// Record how call `id` ended. A failure to record is logged and swallowed:
/// the call has already happened, and its answer is still owed.
pub(super) async fn end(pool: &PgPool, user: UserId, id: i64, outcome: Result<&Reply, String>) {
    let (said, reply, tokens) = match outcome {
        Ok(reply) => (
            "answered",
            reply
                .parts
                .iter()
                .map(|part| part.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            size::reply(reply),
        ),
        Err(refusal) => {
            let tokens = size::text(&refusal);
            ("refused", refusal, tokens)
        }
    };
    let written = async {
        let mut tx = db::scoped(pool, user).await?;
        sqlx::query(
            "UPDATE tool_calls SET outcome = $2, reply = $3, reply_tokens_estimate = $4,
                                   finished_at = now()
             WHERE id = $1",
        )
        .bind(id)
        .bind(said)
        .bind(reply)
        .bind(i64::try_from(tokens).unwrap_or(i64::MAX))
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    };
    if let Err(error) = written.await {
        eprintln!("scorsese-server: could not record how tool call {id} ended: {error}");
    }
}
