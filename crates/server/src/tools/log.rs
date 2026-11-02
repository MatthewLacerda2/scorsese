//! `tool_calls`: every call, whoever made it.
//!
//! Written when a call starts and finished when it answers, so a call that
//! takes the server down with it is on record, unfinished. The reply's words
//! are kept; its pictures are not — they are frames of the edit or of the
//! footage, and either can be drawn again; what both weigh in tokens is
//! kept as an estimate (`size`, #707). An external call carries the API token
//! it came in on, and the client that token's last MCP `initialize` named.
//!
//! A call's arguments are kept up to [`KEPT`] bytes of JSON (#1648); longer
//! ones are kept as their beginning, in an object that says it was cut.

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

/// The most bytes of a call's arguments, written as compact JSON, a row keeps.
pub(super) const KEPT: usize = 16 * 1024;

/// `arguments` as a row keeps them: whole up to [`KEPT`] bytes, and past it
/// `{"cut": true, "bytes": N, "kept": "<their beginning>"}`, itself within
/// [`KEPT`] bytes.
fn kept(arguments: &Value) -> Value {
    let text = arguments.to_string();
    if text.len() <= KEPT {
        return arguments.clone();
    }
    let marker = |end: usize| {
        let mut end = end.min(text.len());
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        serde_json::json!({ "cut": true, "bytes": text.len(), "kept": &text[..end] })
    };
    // Escaping grows the beginning kept, so it is shortened by what the
    // marker overshoots until it fits; each pass shrinks it.
    let mut end = KEPT;
    loop {
        let cut = marker(end);
        let over = cut.to_string().len().saturating_sub(KEPT);
        if over == 0 {
            return cut;
        }
        end = end.saturating_sub(over);
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
    .bind(kept(arguments))
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
        tracing::error!("could not record how tool call {id} ended: {error}");
    }
}

/// Record that call `id` saved the project as `revision` (#1488): what tells
/// an edit that changed something from one that left it as it was. Like
/// [`end`], a failure to record is logged and swallowed.
pub(super) async fn saved(pool: &PgPool, user: UserId, id: i64, revision: i64) {
    let written = async {
        let mut tx = db::scoped(pool, user).await?;
        sqlx::query("UPDATE tool_calls SET saved_revision = $2 WHERE id = $1")
            .bind(id)
            .bind(revision)
            .execute(&mut *tx)
            .await?;
        tx.commit().await
    };
    if let Err(error) = written.await {
        tracing::error!("could not record what tool call {id} saved: {error}");
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{KEPT, kept};

    #[test]
    fn arguments_within_the_bound_are_kept_whole() {
        let small = json!({ "project": 1, "text": "hi" });
        assert_eq!(kept(&small), small);
    }

    #[test]
    fn a_cut_of_escaped_text_still_fits() {
        // Every quote doubles when the text is kept as a string.
        let quoted = json!({ "text": "\"".repeat(KEPT) });
        let cut = kept(&quoted);
        assert!(cut.to_string().len() <= KEPT, "{}", cut.to_string().len());
        assert_eq!(cut["cut"], true);
        assert_eq!(cut["bytes"], quoted.to_string().len());
        let wide = json!({ "text": "é".repeat(KEPT) });
        assert!(kept(&wide).to_string().len() <= KEPT);
    }
}
