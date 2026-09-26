//! A user's projects: list, create, open, save, rename, delete.
//!
//! Deliberately few. What a project *says* is changed by `scorsese-core`'s
//! editing functions, reached through the tools (#539, #540) or the editor
//! (#545) — not by an endpoint per operation here. These are the storage verbs
//! those callers are built on, and each one is a call into
//! [`crate::projects`], which has the argument for the revision rule.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use scorsese_core::{Fps, Project};
use serde::Deserialize;
use serde_json::{Value, json};

use super::AppState;
use super::auth::Member;
use super::error::ApiError;
use crate::projects::{self, Stored, Summary};

/// `GET /api/projects`: the caller's projects, without their documents.
pub async fn list(
    State(state): State<AppState>,
    member: Member,
) -> Result<Json<Vec<Summary>>, ApiError> {
    Ok(Json(projects::list(&state.pool, member.user).await?))
}

/// `POST /api/projects`'s body.
#[derive(Debug, Deserialize)]
pub struct NewProject {
    /// What to call it.
    pub name: String,
    /// The timeline grid, spelled as `project.json` spells `timeline_fps`.
    /// The document's own default when omitted.
    #[serde(default)]
    pub fps: Option<Fps>,
}

/// `POST /api/projects`: an empty project, made by `core` exactly as
/// `scorsese new` makes one.
pub async fn create(
    State(state): State<AppState>,
    member: Member,
    Json(new): Json<NewProject>,
) -> Result<(StatusCode, Json<Stored>), ApiError> {
    let name = named(&new.name)?;
    let project = Project::new(name, new.fps.unwrap_or_default());
    let summary = projects::create(&state.pool, member.user, &project).await?;
    Ok((
        StatusCode::CREATED,
        Json(Stored {
            summary,
            document: project,
        }),
    ))
}

/// `GET /api/projects/{id}`: the project, its document, and the revision a
/// save of that document has to name.
pub async fn open(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
) -> Result<Json<Stored>, ApiError> {
    Ok(Json(projects::open(&state.pool, member.user, id).await?))
}

/// `PUT /api/projects/{id}`'s body.
#[derive(Debug, Deserialize)]
pub struct Save {
    /// The revision the document was read at.
    pub revision: i64,
    /// The whole new document.
    pub document: Value,
}

/// `PUT /api/projects/{id}`: replace the document, if nobody wrote since
/// `revision`. `409` if somebody did, and nothing is written.
///
/// Parsed by the strict path, so a document in another build's format or
/// with an unknown field is a `400` — the same refusal `Project::load` gives.
/// Not validated, exactly as a local save is not: an edit in progress may be
/// incoherent for a moment, and validating is the render's job.
pub async fn save(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
    Json(save): Json<Save>,
) -> Result<Json<Value>, ApiError> {
    let project = Project::from_json(&save.document.to_string())
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    let revision = projects::save(&state.pool, member.user, id, save.revision, &project).await?;
    Ok(Json(json!({ "revision": revision })))
}

/// `PATCH /api/projects/{id}`'s body.
#[derive(Debug, Deserialize)]
pub struct Rename {
    /// The new name.
    pub name: String,
}

/// `PATCH /api/projects/{id}`: rename. The name lives in the document, so
/// this is an edit like any other — made under the row's lock, so it needs
/// no revision and cannot conflict.
pub async fn rename(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
    Json(rename): Json<Rename>,
) -> Result<Json<Value>, ApiError> {
    let name = named(&rename.name)?;
    let ((), revision) = projects::edit(&state.pool, member.user, id, |project| {
        project.name = name;
    })
    .await?;
    Ok(Json(json!({ "revision": revision })))
}

/// `DELETE /api/projects/{id}`.
pub async fn delete(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    if projects::delete(&state.pool, member.user, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}

/// A name with something in it, trimmed.
fn named(name: &str) -> Result<String, ApiError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ApiError::BadRequest("a project needs a name".to_owned()));
    }
    Ok(name.to_owned())
}
