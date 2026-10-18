//! What the person changed by hand since a turn began, read from the editor's
//! rows of `tool_calls` (#545): the one record of an edit nobody asked the
//! assistant for.

use crate::db::Tx;

/// Whether the person rewrote `project`'s script from the editor (#1445)
/// since turn `turn` began — or, when it was told of a rewrite as it resumed,
/// since then (#1488) — so the model's memory of it may be stale.
///
/// A rewrite during the turn counts too: the turn may have read the script
/// before it landed. Only a write that saved a new revision counts: one that
/// left the script as it was changed nothing the model remembers.
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
               AND c.saved_revision IS NOT NULL
               AND c.started_at >= coalesce(t.script_told_at, t.started_at))",
    )
    .bind(project)
    .bind(turn)
    .fetch_one(&mut **tx)
    .await
}

/// For paused turn `turn` as it resumes on a question's or a quote's answer:
/// whether the person rewrote the script meanwhile, and if so, that it has now
/// been told — so the turn after it is not told of the same rewrite again.
pub(in crate::assistant) async fn told_resuming(
    tx: &mut Tx,
    project: i64,
    turn: i64,
) -> Result<bool, sqlx::Error> {
    if !rewrote_script(tx, project, turn).await? {
        return Ok(false);
    }
    sqlx::query("UPDATE chat_turns SET script_told_at = now() WHERE id = $1")
        .bind(turn)
        .execute(&mut **tx)
        .await?;
    Ok(true)
}
