//! How hard a project's assistant thinks (#769, #1443): the choice, kept on
//! the project beside its model, and the level the next turn runs at.
//!
//! The choice is a routing flag, like the model ([`super::model`]): changing
//! it rewrites no turn. A turn at a level other than the one in effect opens
//! with an effort change in the conversation itself
//! (`scorsese_providers::chat::Effort::change`), so the history before it
//! still reads from the cache — changing the request's top-level effort would
//! miss it on every message.

use scorsese_providers::chat::Effort;

use super::AssistantError;
use crate::db::{self, Tx, UserId};
use crate::http::AppState;

/// The level `project`'s next turn runs at, in the caller's transaction.
/// `NotFound` when the project is not theirs.
pub(super) async fn of(tx: &mut Tx, project: i64) -> Result<Effort, AssistantError> {
    let name: Option<String> =
        sqlx::query_scalar("SELECT assistant_effort FROM projects WHERE id = $1")
            .bind(project)
            .fetch_optional(&mut **tx)
            .await?;
    let name = name.ok_or(AssistantError::NotFound)?;
    Effort::from_name(&name).ok_or_else(|| {
        AssistantError::Internal(format!(
            "project {project} names an unknown effort, {name:?}"
        ))
    })
}

/// Set the level `user`'s `project` runs at to the one named `name`. Takes
/// effect from the next turn; one already running, or resuming after a
/// question or a quote, keeps the level it started at.
pub async fn choose_effort(
    state: &AppState,
    user: UserId,
    project: i64,
    name: &str,
) -> Result<Effort, AssistantError> {
    let effort = Effort::from_name(name).ok_or_else(|| {
        let offered: Vec<&str> = Effort::ALL.iter().map(|effort| effort.as_str()).collect();
        AssistantError::Invalid(format!(
            "{name:?} is not an effort the assistant offers; it offers {}",
            offered.join(", ")
        ))
    })?;
    let mut tx = db::scoped(&state.pool, user).await?;
    let changed = sqlx::query("UPDATE projects SET assistant_effort = $2 WHERE id = $1")
        .bind(project)
        .bind(effort.as_str())
        .execute(&mut *tx)
        .await?;
    if changed.rows_affected() == 0 {
        return Err(AssistantError::NotFound);
    }
    tx.commit().await?;
    Ok(effort)
}
