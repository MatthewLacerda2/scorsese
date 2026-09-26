//! Serving a file in the byte range the request asks for.
//!
//! A `<video>` element asks for its source in pieces — the start, to read the
//! header, then wherever the viewer seeks — and plays as soon as the first
//! piece arrives. Without ranges it would have to fetch the whole file first,
//! and a screen recording is gigabytes (#527: "video streams in pieces").
//!
//! One range per request, which is what media elements send; a request for
//! several gets the whole file, which the specification allows. Written here
//! rather than taken from a crate: it is a few dozen lines, and the crates
//! that do it bring a whole static-file server with them.
//!
//! A library file never changes under its id — it is named by its hash — so it
//! is sent as cacheable forever, privately (it is one person's), tagged by the
//! hash.

use std::io::SeekFrom;
use std::path::Path;

use axum::body::Body;
use axum::http::header::{
    ACCEPT_RANGES, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, ETAG, RANGE,
    X_CONTENT_TYPE_OPTIONS,
};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use futures_util::stream;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use super::error::ApiError;

/// How much of the file is read per piece of the response body.
const PIECE: usize = 256 * 1024;

/// A file to serve, and what to say about it.
#[derive(Debug)]
pub(super) struct Served<'a> {
    /// Where it is.
    pub(super) path: &'a Path,
    /// Its `Content-Type`.
    pub(super) content_type: &'a str,
    /// A tag for its content, which never changes under its URL.
    pub(super) etag: &'a str,
}

/// The file, or the part of it `headers` ask for.
pub(super) async fn serve(served: &Served<'_>, headers: &HeaderMap) -> Result<Response, ApiError> {
    let mut file = match tokio::fs::File::open(served.path).await {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // A row whose file is gone: the disk lost it. Said in the log,
            // because nothing the user does brings it back.
            eprintln!("scorsese-server: {} is missing", served.path.display());
            return Err(ApiError::NotFound);
        }
        Err(error) => {
            eprintln!("scorsese-server: {}: {error}", served.path.display());
            return Err(ApiError::Internal);
        }
    };
    let size = file
        .metadata()
        .await
        .map_err(|error| {
            eprintln!("scorsese-server: {}: {error}", served.path.display());
            ApiError::Internal
        })?
        .len();

    let asked = headers.get(RANGE).and_then(|value| value.to_str().ok());
    let (status, start, length) = match asked.map(|value| range(value, size)) {
        None | Some(Ask::Whole) => (StatusCode::OK, 0, size),
        Some(Ask::Part(start, end)) => (StatusCode::PARTIAL_CONTENT, start, end - start + 1),
        Some(Ask::Unsatisfiable) => {
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::RANGE_NOT_SATISFIABLE;
            insert(&mut response, CONTENT_RANGE, &format!("bytes */{size}"));
            return Ok(response);
        }
    };
    file.seek(SeekFrom::Start(start)).await.map_err(|_| ApiError::Internal)?;

    let pieces = stream::unfold((file, length), |(mut file, left)| async move {
        if left == 0 {
            return None;
        }
        let mut buffer = vec![0; PIECE.min(usize::try_from(left).unwrap_or(PIECE))];
        match file.read(&mut buffer).await {
            Ok(0) => None,
            Ok(read) => {
                buffer.truncate(read);
                Some((Ok(buffer), (file, left - read as u64)))
            }
            Err(error) => Some((Err(error), (file, 0))),
        }
    });
    let mut response = Response::new(Body::from_stream(pieces));
    *response.status_mut() = status;
    insert(&mut response, CONTENT_TYPE, served.content_type);
    insert(&mut response, CONTENT_LENGTH, &length.to_string());
    insert(&mut response, ACCEPT_RANGES, "bytes");
    insert(&mut response, ETAG, &format!("\"{}\"", served.etag));
    insert(&mut response, CACHE_CONTROL, "private, max-age=31536000, immutable");
    insert(&mut response, X_CONTENT_TYPE_OPTIONS, "nosniff");
    if status == StatusCode::PARTIAL_CONTENT {
        let end = start + length - 1;
        insert(&mut response, CONTENT_RANGE, &format!("bytes {start}-{end}/{size}"));
    }
    Ok(response)
}

fn insert(response: &mut Response, name: axum::http::HeaderName, value: &str) {
    if let Ok(value) = HeaderValue::from_str(value) {
        response.headers_mut().insert(name, value);
    }
}

/// What a `Range` header asks of a file of `size` bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ask {
    /// All of it: no single byte range was asked for in a form understood.
    Whole,
    /// Bytes `start..=end`.
    Part(u64, u64),
    /// A range that starts past the end.
    Unsatisfiable,
}

/// Read `bytes=a-b`, `bytes=a-` or `bytes=-n` against a file of `size` bytes.
pub(crate) fn range(header: &str, size: u64) -> Ask {
    let Some(spec) = header.trim().strip_prefix("bytes=") else {
        return Ask::Whole;
    };
    if spec.contains(',') {
        return Ask::Whole;
    }
    let Some((first, last)) = spec.trim().split_once('-') else {
        return Ask::Whole;
    };
    let (first, last) = (first.trim(), last.trim());
    let number = |text: &str| text.parse::<u64>().ok();
    let (start, end) = match (first.is_empty(), last.is_empty()) {
        // The last n bytes.
        (true, false) => match number(last) {
            Some(0) => return Ask::Unsatisfiable,
            Some(n) => (size.saturating_sub(n), size.saturating_sub(1)),
            None => return Ask::Whole,
        },
        (false, _) => match (number(first), last.is_empty().then_some(u64::MAX).or_else(|| number(last))) {
            (Some(start), Some(end)) if start <= end => (start, end.min(size.saturating_sub(1))),
            _ => return Ask::Whole,
        },
        (true, true) => return Ask::Whole,
    };
    if size == 0 || start >= size {
        Ask::Unsatisfiable
    } else {
        Ask::Part(start, end)
    }
}
