//! Resumable uploads: the subset of tus 1.0.0 the web app's Uppy speaks.
//!
//! **Why tus.** The Cloudflare tunnel in front of the server refuses a request
//! body over 100 MB, and a screen recording is several times that; a file has
//! to arrive in chunks, and a dropped connection must resume from the last one
//! rather than start over. tus is the open protocol for exactly that, and Uppy
//! — the upload widget the web app uses (#527) — speaks it out of the box.
//!
//! **Why written here.** The Rust tus servers there are are applications
//! (`rustus`), not libraries an axum router can mount, and what a browser
//! actually needs is four requests: *creation* (`POST`, announce a file and
//! get its URL), `HEAD` (how far did it get), `PATCH` (the next chunk, at the
//! offset `HEAD` said) and *termination* (`DELETE`). That is the core protocol
//! plus two extensions, a page of code whose storage half is
//! [`crate::library::upload`]. Not supported: *creation-with-upload*,
//! *deferred length*, *concatenation*, *checksum* (the whole file's hash is
//! checked at the end instead) and *expiration* headers (an upload untouched
//! for a day is swept).
//!
//! **What a client sends.** `Upload-Metadata` must carry `filename` (Uppy's
//! `name` works too) and `sha256`, the hex SHA-256 the browser computed —
//! which is also what lets it ask `GET /api/library?sha256=` before sending a
//! byte. Every response carries `Tus-Resumable: 1.0.0`. The last `PATCH` names
//! the new library item in `Scorsese-Library-Item`.
//!
//! **Retries.** A tus client retries a `409` or `423` by default. `423` means
//! another request is writing the upload, which a retry is the answer to; `409`
//! is a chunk at the wrong offset, which the client answers by asking `HEAD` —
//! and is also "you already have this file", whose body carries the `item`. The
//! web app's `onShouldRetry` stops on the latter.

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, LOCATION};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use super::AppState;
use super::auth::Member;
use super::error::ApiError;
use crate::library::{Announced, Appended, MAX_UPLOAD_BYTES};

/// The one tus version spoken.
const VERSION: &str = "1.0.0";

/// The header naming the library item the last chunk made.
pub const LIBRARY_ITEM: &str = "scorsese-library-item";

/// `OPTIONS /api/uploads`: what this server supports. Needs no login; it says
/// nothing about anybody.
pub async fn options() -> Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    set(&mut response, "tus-extension", "creation,termination");
    set(&mut response, "tus-max-size", &MAX_UPLOAD_BYTES.to_string());
    tus(response)
}

/// `POST /api/uploads`: announce an upload; `201` and its URL in `Location`.
pub async fn announce(
    State(state): State<AppState>,
    member: Member,
    headers: HeaderMap,
) -> Response {
    let announced = match resumable(&headers).and_then(|()| announcement(&headers)) {
        Ok(announced) => announced,
        Err(refused) => return tus(refused.into_response()),
    };
    tus(
        match state.library.start_upload(member.user, &announced).await {
            Ok(id) => {
                let mut response = StatusCode::CREATED.into_response();
                set(
                    &mut response,
                    LOCATION.as_str(),
                    &format!("/api/uploads/{id}"),
                );
                response
            }
            Err(error) => ApiError::from(error).into_response(),
        },
    )
}

/// `HEAD /api/uploads/{id}`: how far it got.
pub async fn progress(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
) -> Response {
    tus(match state.library.upload_progress(member.user, id).await {
        Ok(progress) => {
            let mut response = StatusCode::OK.into_response();
            set(&mut response, "upload-offset", &progress.offset.to_string());
            set(&mut response, "upload-length", &progress.length.to_string());
            set(&mut response, CACHE_CONTROL.as_str(), "no-store");
            response
        }
        Err(error) => ApiError::from(error).into_response(),
    })
}

/// `PATCH /api/uploads/{id}`: the next chunk, starting at `Upload-Offset`.
pub async fn append(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
    headers: HeaderMap,
    body: Body,
) -> Response {
    if let Err(refused) = resumable(&headers) {
        return tus(refused.into_response());
    }
    if header(&headers, CONTENT_TYPE.as_str()) != Some("application/offset+octet-stream") {
        let why = "a chunk is sent as application/offset+octet-stream";
        return tus(refused(StatusCode::UNSUPPORTED_MEDIA_TYPE, why).into_response());
    }
    let Some(offset) = header(&headers, "upload-offset").and_then(|v| v.parse().ok()) else {
        return tus(refused(StatusCode::BAD_REQUEST, "Upload-Offset is missing").into_response());
    };
    let appended = state
        .library
        .append(member.user, id, offset, body.into_data_stream())
        .await;
    tus(match appended {
        Ok(Appended::Partial(progress)) => {
            let mut response = StatusCode::NO_CONTENT.into_response();
            set(&mut response, "upload-offset", &progress.offset.to_string());
            response
        }
        Ok(Appended::Finished(item)) => {
            let mut response = StatusCode::NO_CONTENT.into_response();
            set(&mut response, "upload-offset", &item.size_bytes.to_string());
            set(&mut response, LIBRARY_ITEM, &item.id.to_string());
            response
        }
        Err(error) => ApiError::from(error).into_response(),
    })
}

/// `DELETE /api/uploads/{id}`: abandon it.
pub async fn cancel(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
) -> Response {
    tus(match state.library.cancel_upload(member.user, id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => ApiError::from(error).into_response(),
    })
}

/// Refused unless the request speaks tus 1.0.0.
fn resumable(headers: &HeaderMap) -> Result<(), ApiError> {
    if header(headers, "tus-resumable") == Some(VERSION) {
        return Ok(());
    }
    Err(refused(
        StatusCode::PRECONDITION_FAILED,
        "this server speaks tus 1.0.0: send Tus-Resumable: 1.0.0",
    ))
}

/// What a creation request announces.
fn announcement(headers: &HeaderMap) -> Result<Announced, ApiError> {
    let length = header(headers, "upload-length")
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| refused(StatusCode::BAD_REQUEST, "Upload-Length is missing"))?;
    let metadata = metadata(header(headers, "upload-metadata").unwrap_or_default());
    let find = |keys: &[&str]| {
        metadata
            .iter()
            .find(|(key, _)| keys.contains(&key.as_str()))
            .map(|(_, value)| value.clone())
    };
    let name = find(&["filename", "name"])
        .ok_or_else(|| refused(StatusCode::BAD_REQUEST, "Upload-Metadata names no filename"))?;
    let sha256 = find(&["sha256"]).ok_or_else(|| {
        refused(
            StatusCode::BAD_REQUEST,
            "Upload-Metadata carries no sha256: hash the file before sending it",
        )
    })?;
    Ok(Announced {
        name,
        sha256,
        length,
    })
}

/// `key base64,key base64` → pairs. A pair that does not decode is dropped.
fn metadata(header: &str) -> Vec<(String, String)> {
    header
        .split(',')
        .filter_map(|pair| {
            let mut parts = pair.split_whitespace();
            let key = parts.next()?.to_owned();
            let value = STANDARD.decode(parts.next().unwrap_or_default()).ok()?;
            Some((key, String::from_utf8(value).ok()?))
        })
        .collect()
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

fn refused(status: StatusCode, why: &str) -> ApiError {
    ApiError::Refused {
        status,
        message: why.to_owned(),
        detail: serde_json::json!({}),
    }
}

fn set(response: &mut Response, name: &str, value: &str) {
    if let (Ok(name), Ok(value)) = (
        axum::http::HeaderName::try_from(name),
        HeaderValue::from_str(value),
    ) {
        response.headers_mut().insert(name, value);
    }
}

/// Every tus response says which protocol it speaks — and which it would,
/// which is what a `412` owes a client that sent another.
fn tus(mut response: Response) -> Response {
    set(&mut response, "tus-resumable", VERSION);
    set(&mut response, "tus-version", VERSION);
    response
}
