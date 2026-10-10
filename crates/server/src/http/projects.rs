//! A user's projects: list, create, open, save, rename, delete — and change
//! what one is made for.
//!
//! Deliberately few. What a project *says* is changed by `scorsese-core`'s
//! editing functions, reached through the tools (#539, #540) or the editor
//! (#545) — not by an endpoint per operation here. These are the storage verbs
//! those callers are built on, and each one is a call into
//! [`crate::projects`], which has the argument for the revision rule.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use scorsese_core::style::{Platform, Start};
use scorsese_core::{Fps, Project};
use serde::Deserialize;
use serde_json::{Value, json};

use super::AppState;
use super::auth::Member;
use super::error::ApiError;
use crate::assistant::{self, Retargeted};
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
    /// The placement the video is made for (#1016), by id; none when omitted.
    #[serde(default)]
    pub platform: Option<String>,
    /// The kind of video it is, by style id; none when omitted. Refused when
    /// it is not made for `platform`.
    #[serde(default)]
    pub style: Option<String>,
    /// Library files to bring in at once, by id — what `import` takes.
    #[serde(default)]
    pub assets: Vec<i64>,
}

/// `POST /api/projects`: a project, made by `core` exactly as `scorsese new`
/// makes one — with the starting brief in its script when a platform or a
/// style was chosen (#1016), and the library files asked for imported.
///
/// `400` for a platform or style the library does not have, a style not made
/// for the platform, or a file that is not in the caller's library; nothing
/// is kept then, so a refused create never leaves a half-made project.
pub async fn create(
    State(state): State<AppState>,
    member: Member,
    Json(new): Json<NewProject>,
) -> Result<(StatusCode, Json<Stored>), ApiError> {
    let name = named(&new.name)?;
    let start = started(new.platform.as_deref(), new.style.as_deref())?;
    let project = Project::new(name, new.fps.unwrap_or_default());
    let stored = projects::begin(&state.pool, member.user, project, &start).await?;
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

/// `PUT /api/projects/{id}/start`'s body: what the project is made for now.
#[derive(Debug, Deserialize)]
pub struct Retarget {
    /// The placement, by id; none when omitted.
    #[serde(default)]
    pub platform: Option<String>,
    /// The kind of video, by style id; none when omitted.
    #[serde(default)]
    pub style: Option<String>,
    /// The person's words for the turn that carries the change, in their own
    /// language — the web app writes them. A plain default when omitted.
    #[serde(default)]
    pub message: Option<String>,
}

/// `PUT /api/projects/{id}/start`: change the platform or the style a project
/// is made for (#1016). The choice is kept and, when it changed, the assistant
/// is told in a turn of its own, the way a confirmed quote is: it updates the
/// script, which the server never rewrites. `400` for a choice the library
/// refuses; `404` for a project that is not theirs.
pub async fn retarget(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
    Json(retarget): Json<Retarget>,
) -> Result<Json<Retargeted>, ApiError> {
    let start = started(retarget.platform.as_deref(), retarget.style.as_deref())?;
    let words = retarget
        .message
        .as_deref()
        .map(str::trim)
        .filter(|words| !words.is_empty())
        .unwrap_or("I changed what this video is made for.");
    Ok(Json(
        assistant::retarget(&state, member.user, id, start, words).await?,
    ))
}

/// A platform and a style as the library takes them, or why not.
fn started(platform: Option<&str>, style: Option<&str>) -> Result<Start, ApiError> {
    let platform = platform
        .map(str::parse::<Platform>)
        .transpose()
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    Start::new(platform, style).map_err(|error| ApiError::BadRequest(error.to_string()))
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
