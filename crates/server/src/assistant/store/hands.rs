//! What the person changed by hand since a turn began, read from the editor's
//! rows of `tool_calls` (#545): the one record of an edit nobody asked the
//! assistant for.

use crate::db::Tx;

/// The editor's script rewrites on `$1` that turn `$2` has not been told of:
/// since the turn began, or, once it was told as it resumed, since the newest
/// rewrite it was told of (#1493). The bound is the rewrite itself, never a
/// clock, so a write that lands while the telling commits is neither told
/// twice nor missed.
macro_rules! untold {
    () => {
        "FROM tool_calls c, chat_turns t
         WHERE t.id = $2 AND c.project_id = $1 AND c.client = 'editor'
           AND c.tool = 'script_write' AND c.outcome = 'answered'
           AND c.saved_revision IS NOT NULL
           AND CASE WHEN t.script_told_at IS NULL THEN c.started_at >= t.started_at
                    ELSE c.started_at > t.script_told_at END"
    };
}

/// Whether the person rewrote `project`'s script from the editor (#1445)
/// since turn `turn` began — or, when it was told of a rewrite as it resumed,
/// since that rewrite (#1488) — so the model's memory of it may be stale.
///
/// A rewrite during the turn counts too: the turn may have read the script
/// before it landed. Only a write that saved a new revision counts: one that
/// left the script as it was changed nothing the model remembers.
pub(in crate::assistant) async fn rewrote_script(
    tx: &mut Tx,
    project: i64,
    turn: i64,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(concat!("SELECT EXISTS (SELECT 1 ", untold!(), ")"))
        .bind(project)
        .bind(turn)
        .fetch_one(&mut **tx)
        .await
}

/// For paused turn `turn` as it resumes on a question's or a quote's answer:
/// whether the person rewrote the script meanwhile, and if so, that it has now
/// been told — up to the newest rewrite it saw, so the turn after it is not
/// told of the same rewrite again.
pub(in crate::assistant) async fn told_resuming(
    tx: &mut Tx,
    project: i64,
    turn: i64,
) -> Result<bool, sqlx::Error> {
    let told = sqlx::query(concat!(
        "UPDATE chat_turns u SET script_told_at = n.newest
         FROM (SELECT max(c.started_at) AS newest ",
        untold!(),
        ") n WHERE u.id = $2 AND n.newest IS NOT NULL"
    ))
    .bind(project)
    .bind(turn)
    .execute(&mut **tx)
    .await?;
    Ok(told.rows_affected() > 0)
}
