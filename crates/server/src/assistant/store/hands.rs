//! What the person changed by hand since a turn began, read from the editor's
//! rows of `tool_calls` (#545): the one record of an edit nobody asked the
//! assistant for.

use crate::db::Tx;

/// Whether the person rewrote `project`'s script from the editor (#1445)
/// after turn `turn` began — so the model's memory of it may be stale.
///
/// A rewrite during the turn counts too: the turn may have read the script
/// before it landed.
pub(in crate::assistant) async fn rewrote_script(
    tx: &mut Tx,
    project: i64,
    turn: i64,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM tool_calls c, chat_turns t
             WHERE t.id = $2 AND c.project_id = $1 AND c.client = 'editor'
               AND c.tool = 'script_write' AND c.outcome = 'answered'
               AND c.started_at >= t.started_at)",
    )
    .bind(project)
    .bind(turn)
    .fetch_one(&mut **tx)
    .await
}
