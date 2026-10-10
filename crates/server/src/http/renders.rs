//! Finished renders: ask for one, list a project's, download one.
//!
//! Asking is answered at once either way — `200` with the render when the
//! same document in the same shape is already in the cache, `202` with the
//! job making it otherwise — and the job's progress arrives on
//! `GET /api/events` like every other job's. [`crate::renders`] has the
//! argument for the key, the quota and eviction.

use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::header::CONTENT_DISPOSITION;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use futures_util::StreamExt;
use serde_json::{Value, json};

use super::AppState;
use super::auth::Member;
use super::error::ApiError;
use super::ranges;
use crate::projects;
use crate::renders::request::{self, AskError, Asked};
use crate::renders::{Ask, PreviewAsk, RenderView, Settings, store};

/// `POST /api/projects/{id}/renders`: the render of the project as it is now,
/// in the shape asked for — kept (`200`) or on its way (`202`).
///
/// A picture with no size asked for is delivered at the size of the platform
/// the project is made for (#1016), when it names one.
pub async fn request(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
    Json(mut ask): Json<Ask>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    // First, before the project is read: a shape we do not write is the
    // cheapest thing to refuse, as it is for `scorsese render`.
    let mut settings = Settings::from_ask(&ask).map_err(ApiError::BadRequest)?;
    if ask.resolution.is_none() && settings.resolution.is_some() {
        ask.resolution = platform_size(&state, &member, id).await?;
        settings = Settings::from_ask(&ask).map_err(ApiError::BadRequest)?;
    }
    answer(&state, &member, id, settings).await
}

/// The delivery size of the platform project `id` is made for, as
/// `WIDTHxHEIGHT` — `None` when it names none.
async fn platform_size(
    state: &AppState,
    member: &Member,
    id: i64,
) -> Result<Option<String>, ApiError> {
    let stored = projects::open(&state.pool, member.user, id).await?;
    Ok(stored.summary.platform.map(|platform| {
        let (width, height) = platform.size();
        format!("{width}x{height}")
    }))
}

/// `POST /api/projects/{id}/previews`: the editor's preview video of the
/// project as it is now (#542) — a render at a preview quality, answered in
/// the same shape as a render — at the platform's size, like a render, when
/// none is asked for.
pub async fn preview(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
    Json(mut ask): Json<PreviewAsk>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    if ask.resolution.is_none() {
        ask.resolution = platform_size(&state, &member, id).await?;
    }
    let settings = Settings::from_preview(&ask).map_err(ApiError::BadRequest)?;
    answer(&state, &member, id, settings).await
}

/// Kept (`200`) or on its way (`202`): one answer for both routes.
async fn answer(
    state: &AppState,
    member: &Member,
    id: i64,
    settings: Settings,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let asked = request::ask(
        &state.pool,
        &state.jobs,
        &state.renders,
        member.user,
        id,
        settings,
    )
    .await
    .map_err(|error| match error {
        AskError::Invalid(why) => ApiError::BadRequest(why),
        AskError::Project(error) => error.into(),
        AskError::Database(error) => error.into(),
    })?;
    Ok(match asked {
        Asked::Kept(view) => (
            StatusCode::OK,
            Json(json!({ "render": shown(&view), "job": null })),
        ),
        Asked::Queued(job) => (
            StatusCode::ACCEPTED,
            Json(json!({ "render": null, "job": job })),
        ),
    })
}

/// `GET /api/projects/{id}/renders`: the project's kept renders, most
/// recently used first.
pub async fn list(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
) -> Result<Json<Vec<Value>>, ApiError> {
    let renders = store::list(&state.pool, member.user, id).await?;
    Ok(Json(renders.iter().map(shown).collect()))
}

/// `GET /api/renders/{id}/file`: the file, in the range asked for.
///
/// The row is stamped as used and committed **before** the file is opened,
/// and the file is pinned until the response body is dropped — the two
/// halves of never deleting a file mid-download (`crate::renders`).
pub async fn file(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let download = store::download(&state.pool, member.user, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let relative = std::path::PathBuf::from(&download.path);
    let pin = state.renders.pin(&relative);
    let path = state.renders.absolute(&relative);
    let etag = relative
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    let served = ranges::Served {
        path: &path,
        content_type: download.settings.content_type(),
        etag,
    };
    let response = ranges::serve(&served, &headers).await?;
    let (mut parts, body) = response.into_parts();
    let name = format!(
        "{}.{}",
        filename(&download.name),
        download.settings.extension()
    );
    if let Ok(value) = HeaderValue::from_str(&format!("attachment; filename=\"{name}\"")) {
        parts.headers.insert(CONTENT_DISPOSITION, value);
    }
    // The pin rides inside the body, so it is released when the last byte has
    // gone — or the client went away — and not when this function returns.
    let body = body.into_data_stream().map(move |piece| {
        let _held = &pin;
        piece
    });
    Ok(Response::from_parts(parts, Body::from_stream(body)))
}

/// A render as a response shows it: the row, and where to download it.
fn shown(view: &RenderView) -> Value {
    let mut shown = serde_json::to_value(view).unwrap_or(Value::Null);
    shown["file"] = Value::String(view.file());
    shown
}

/// A project's name as a file name: letters, digits, spaces, `-` and `_`,
/// nothing a header or a file system could read as structure.
fn filename(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let safe = safe.trim();
    if safe.is_empty() {
        "render".to_owned()
    } else {
        safe.to_owned()
    }
}
