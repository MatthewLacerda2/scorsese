//! Which model a project's assistant runs on (#705): the choice, kept on the
//! project, and the list the chat panel's picker shows.
//!
//! The choice is a routing flag and nothing more. Changing it rewrites no
//! turn and locks none: the next turn runs on whatever the project names when
//! it starts, and builds its history from the record of every earlier turn
//! (the module doc of [`super`]).

use scorsese_providers::chat::{Model, Vendor};
use serde::Serialize;

use super::{Assistant, AssistantError};
use crate::db::{self, Tx, UserId};
use crate::http::AppState;

/// One model as the picker lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Choice {
    /// The id the project stores and the API takes.
    pub id: &'static str,
    /// Its name, as a person reads it.
    pub label: &'static str,
    /// `anthropic` or `google`.
    pub vendor: &'static str,
    /// Why a turn on it would be refused right now — the server's own words,
    /// shown as they are — or `null` when it can answer.
    pub unavailable: Option<String>,
    /// How long, in seconds after the last answer, a switch away from it can
    /// still miss a warm cache: what the switch warning waits out.
    pub cache_seconds: u64,
}

/// Every model offered, in the picker's order, as `assistant` can reach them.
pub(super) fn choices(assistant: &Assistant) -> Vec<Choice> {
    Model::ALL
        .into_iter()
        .map(|model| Choice {
            id: model.id(),
            label: model.label(),
            vendor: match model.vendor() {
                Vendor::Anthropic => "anthropic",
                Vendor::Google => "google",
            },
            unavailable: if assistant.available(model) {
                None
            } else {
                assistant.chat(model).err().map(|error| error.to_string())
            },
            cache_seconds: model.cache_lifetime().as_secs(),
        })
        .collect()
}

/// The model `project` runs on, in the caller's transaction. `NotFound` when
/// the project is not theirs.
pub(super) async fn of(tx: &mut Tx, project: i64) -> Result<Model, AssistantError> {
    let id: Option<String> =
        sqlx::query_scalar("SELECT assistant_model FROM projects WHERE id = $1")
            .bind(project)
            .fetch_optional(&mut **tx)
            .await?;
    let id = id.ok_or(AssistantError::NotFound)?;
    Model::from_id(&id).ok_or_else(|| {
        AssistantError::Internal(format!("project {project} names an unknown model, {id:?}"))
    })
}

/// Set the model `user`'s `project` runs on to the one with id `id`. Takes
/// effect from the next turn; one already running finishes on the model it
/// started on.
pub async fn choose(
    state: &AppState,
    user: UserId,
    project: i64,
    id: &str,
) -> Result<Choice, AssistantError> {
    let model = Model::from_id(id).ok_or_else(|| {
        let offered: Vec<&str> = Model::ALL.iter().map(|model| model.id()).collect();
        AssistantError::Invalid(format!(
            "{id:?} is not a model the assistant offers; it offers {}",
            offered.join(", ")
        ))
    })?;
    let mut tx = db::scoped(&state.pool, user).await?;
    let changed = sqlx::query("UPDATE projects SET assistant_model = $2 WHERE id = $1")
        .bind(project)
        .bind(model.id())
        .execute(&mut *tx)
        .await?;
    if changed.rows_affected() == 0 {
        return Err(AssistantError::NotFound);
    }
    tx.commit().await?;
    choices(&state.assistant)
        .into_iter()
        .find(|choice| choice.id == model.id())
        .ok_or(AssistantError::NotFound)
}
