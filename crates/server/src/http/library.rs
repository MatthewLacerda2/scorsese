//! A user's library over HTTP: list, details, rename and describe, delete,
//! open, thumbnail. Each a call into [`crate::library`], which has the
//! arguments; uploads are [`super::uploads`].

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use serde::Serialize;
use serde_json::{Value, json};

use super::AppState;
use super::auth::Member;
use super::error::ApiError;
use super::ranges;
use crate::library::{Change, Filter, Item, Kind, Summary, UsedBy};

/// A tile in a list: name, kind, size, and where its thumbnail is.
#[derive(Debug, Serialize)]
pub struct Tile {
    /// Everything but the thumbnail.
    #[serde(flatten)]
    pub summary: Summary,
    /// Where to fetch its thumbnail. `404` there means not drawn yet.
    pub thumbnail: String,
}

/// `GET /api/library`: the caller's files, newest first — thumbnail, name,
/// kind and size only (#527). `?kind=` narrows to one kind, `?search=` to
/// names containing it, `?sha256=` to the file with those bytes: what the
/// browser asks before uploading one.
pub async fn list(
    State(state): State<AppState>,
    member: Member,
    Query(filter): Query<Filter>,
) -> Result<Json<Vec<Tile>>, ApiError> {
    let items = state.library.list(member.user, &filter).await?;
    Ok(Json(
        items
            .into_iter()
            .map(|summary| Tile {
                thumbnail: format!("/api/library/{}/thumbnail", summary.id),
                summary,
            })
            .collect(),
    ))
}

/// One file with everything known about it.
#[derive(Debug, Serialize)]
pub struct Details {
    /// The item.
    #[serde(flatten)]
    pub item: Item,
    /// Whether a generation made it (it has a brief hash).
    pub generated: bool,
    /// The projects that use it — and so keep it from being deleted.
    pub used_by: Vec<UsedBy>,
    /// For a generated file, the record of what made it and what it cost
    /// ([`Library::generation`](crate::library::Library::generation)).
    pub generation: Option<Value>,
}

/// `GET /api/library/{id}`: one file's details.
pub async fn details(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
) -> Result<Json<Details>, ApiError> {
    let item = state.library.get(member.user, id).await?;
    let used_by = state.library.used_by(member.user, id).await?;
    let generation = state.library.generation(member.user, id).await?;
    Ok(Json(Details {
        generated: item.brief_hash.is_some(),
        item,
        used_by,
        generation,
    }))
}

/// `PATCH /api/library/{id}`: `{name?, description?}`; an empty description
/// removes it. The file itself never changes.
pub async fn update(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
    Json(change): Json<Change>,
) -> Result<Json<Item>, ApiError> {
    Ok(Json(state.library.update(member.user, id, &change).await?))
}

/// `DELETE /api/library/{id}`: `204`, or `409` naming the projects that use
/// the file.
pub async fn delete(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    state.library.delete(member.user, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/library/{id}/file`: the file itself, in the byte range asked for
/// — so a video starts playing, and seeks, without being fetched whole.
pub async fn file(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let item = state.library.get(member.user, id).await?;
    let path = state
        .library
        .storage()
        .library_file(member.user, &item.sha256, &item.extension);
    let served = ranges::Served {
        path: &path,
        content_type: item.kind.content_type(&item.extension),
        etag: &item.sha256,
    };
    ranges::serve(&served, &headers).await
}

/// `GET /api/library/{id}/thumbnail`: the picture, or `404` while it is being
/// drawn — asking is also what gets a missing one drawn again.
pub async fn thumbnail(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let Some(path) = state.library.thumbnail(member.user, id).await? else {
        return Err(ApiError::Refused {
            status: StatusCode::NOT_FOUND,
            message: "the thumbnail is still being drawn".to_owned(),
            detail: json!({ "pending": true }),
        });
    };
    // A thumbnail is a picture whichever kind it shows, named by the source's
    // hash and in the format its extension says.
    let name = |part: Option<&std::ffi::OsStr>| {
        part.and_then(|part| part.to_str())
            .unwrap_or_default()
            .to_owned()
    };
    let content_type = Kind::Image.content_type(&name(path.extension()));
    let etag = name(path.file_stem());
    let served = ranges::Served {
        path: &path,
        content_type,
        etag: &etag,
    };
    ranges::serve(&served, &headers).await
}
