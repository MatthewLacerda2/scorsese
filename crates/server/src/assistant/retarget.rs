//! A project's platform or style changed after it was started (#1016).
//!
//! Handed to the assistant the way a confirmed quote is: the server records
//! the new choice and starts a turn whose note says what changed, with what
//! the new choice asks for — the same words a project started for it carries
//! in its brief. The assistant updates the script; the server never rewrites
//! it, because once written the script is the assistant's text and the
//! person's, and nothing parses it back.

use scorsese_core::style::Start;
use serde::Serialize;

use super::start::{Opening, start};
use super::store::TurnView;
use super::{AssistantError, prompt};
use crate::db::UserId;
use crate::http::AppState;
use crate::projects::{self, ProjectError};

/// What a change of platform or style did.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Retargeted {
    /// The turn started to carry the change into the script — `null` when
    /// nothing changed, or when no turn could start.
    pub turn: Option<TurnView>,
    /// Why no turn started, when the choice changed and none did: the choice
    /// is kept all the same, and the assistant is not told.
    pub note: Option<String>,
}

/// Make `user`'s `project` one for `now`, and when that changes anything, start
/// a turn telling the assistant — with `words` as the person's message.
pub async fn retarget(
    state: &AppState,
    user: UserId,
    project: i64,
    now: Start,
    words: &str,
) -> Result<Retargeted, AssistantError> {
    let before = projects::retarget(&state.pool, user, project, &now)
        .await
        .map_err(|error| match error {
            ProjectError::NotFound => AssistantError::NotFound,
            other => AssistantError::Internal(other.to_string()),
        })?;
    if before == now {
        return Ok(Retargeted {
            turn: None,
            note: None,
        });
    }
    let opening = Opening {
        prompt: words.to_owned(),
        fresh: false,
        notes: vec![prompt::retargeted(&before, &now)],
        effort: None,
    };
    Ok(match start(state, user, project, opening).await {
        Ok(turn) => Retargeted {
            turn: Some(turn),
            note: None,
        },
        Err(error) => Retargeted {
            turn: None,
            note: Some(error.to_string()),
        },
    })
}
