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
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::AppState;
use super::auth::Member;
use super::error::ApiError;
use crate::projects::{self, Stored, Summary};
use crate::tools::Client;

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
    /// Library files to bring in at once, by id — what `import` takes.
    #[serde(default)]
    pub assets: Vec<i64>,
}

/// `POST /api/projects`: a project, made by `core` exactly as `scorsese new`
/// makes one, with the library files asked for imported. No platform or
/// style is asked for (#1531): the assistant reads them from the first prompt.
///
/// `400` for a blank name or a file that is not in the caller's library;
/// nothing is kept then, so a refused create never leaves a half-made project.
pub async fn create(
    State(state): State<AppState>,
    member: Member,
    Json(new): Json<NewProject>,
) -> Result<(StatusCode, Json<Stored>), ApiError> {
    let name = named(&new.name)?;
    let project = Project::new(name, new.fps.unwrap_or_default());
    let summary = projects::create(&state.pool, member.user, &project).await?;
    let stored = Stored {
        summary,
        document: project,
    };
    if new.assets.is_empty() {
        return Ok((StatusCode::CREATED, Json(stored)));
    }
    let id = stored.summary.id;
    let arguments = json!({ "project": id, "items": new.assets });
    let imported = state
        .tools
        .call(member.user, Client::Editor, "import", &arguments)
        .await;
    if let Err(why) = imported {
        projects::delete(&state.pool, member.user, id).await?;
        return Err(ApiError::BadRequest(why));
    }
    let stored = projects::open(&state.pool, member.user, id).await?;
    Ok((StatusCode::CREATED, Json(stored)))
}

/// `GET /api/projects/{id}`'s answer.
#[derive(Debug, Serialize)]
pub struct Opened {
    /// The project and its document.
    #[serde(flatten)]
    pub stored: Stored,
    /// When its script last changed, in microseconds since the Unix epoch, or
    /// `null` without one: the editor's Script modal re-reads the script only
    /// when this moved (#1493).
    pub script_changed_at: Option<i64>,
}

/// `GET /api/projects/{id}`: the project, its document, the revision a save
/// of that document has to name, and when its script last changed.
pub async fn open(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
) -> Result<Json<Opened>, ApiError> {
    let (stored, script_changed_at) =
        projects::open_with_script_stamp(&state.pool, member.user, id).await?;
    Ok(Json(Opened {
        stored,
        script_changed_at,
    }))
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
/// with an unknown field is a `400` — the same refusal `Project::load` gives —
/// and validated as `project_write` validates: a document that does not is a
/// `400` listing every problem in `Project::validate`'s words, and nothing is
/// stored. What a stored document names is laid out on this machine, so one
/// naming a path out of its folder is never kept.
pub async fn save(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
    Json(save): Json<Save>,
) -> Result<Json<Value>, ApiError> {
    let project = Project::from_json(&save.document.to_string())
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    project
        .validate()
        .map_err(|problems| ApiError::BadRequest(problems.to_string()))?;
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
