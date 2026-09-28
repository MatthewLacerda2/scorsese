//! A user's templates (#546): `GET /api/templates` and
//! `DELETE /api/templates/{id}`.
//!
//! Storage verbs only, as for projects: a template is made and used through
//! the tools — `template_save` and `template_insert`, which the assistant, web
//! MCP and the editor's route all reach — so saving one and putting one in a
//! project is `core`'s work in every case. What the page needs besides is the
//! list to choose from and a way to let one go, which is also the way to
//! release a library file only a template still holds.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use super::AppState;
use super::auth::Member;
use super::error::ApiError;
use crate::templates::{self, Summary, TemplateError};

/// `GET /api/templates`: the caller's templates, by name.
pub async fn list(
    State(state): State<AppState>,
    member: Member,
) -> Result<Json<Vec<Summary>>, ApiError> {
    Ok(Json(templates::list(&state.pool, member.user).await?))
}

/// `DELETE /api/templates/{id}`: `204`, or `404` for one that is not theirs.
/// Videos it was inserted into keep their copies.
pub async fn delete(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    if templates::delete(&state.pool, member.user, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}

impl From<TemplateError> for ApiError {
    fn from(error: TemplateError) -> Self {
        match error {
            TemplateError::NotFound => Self::NotFound,
            TemplateError::NameTaken { .. } => Self::Conflict(error.to_string()),
            TemplateError::Unnamed | TemplateError::UnknownFiles(_) => {
                Self::BadRequest(error.to_string())
            }
            TemplateError::Unreadable { .. }
            | TemplateError::Migrate { .. }
            | TemplateError::Serialize(_)
            | TemplateError::Database(_) => {
                eprintln!("scorsese-server: {error}");
                Self::Internal
            }
        }
    }
}
