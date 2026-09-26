//! What a request that did not succeed is answered with.
//!
//! Always `{"error": "…"}` with a status, so the web app and a script read one
//! shape. A failure that is the server's own — the database, the OS — is
//! logged in full and answered with a generic `500`: its detail is for the
//! operator, not for whoever sent the request.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use crate::accounts::AccountError;
use crate::library::LibraryError;
use crate::projects::ProjectError;

/// A request refused or failed, and what to tell its sender.
#[derive(Debug)]
pub enum ApiError {
    /// Not logged in, or the credential presented is not valid.
    Unauthorized(&'static str),
    /// Logged in, and not allowed to do this with that credential.
    Forbidden(&'static str),
    /// There is nothing by that id — or nothing of *theirs*, which is
    /// deliberately the same answer.
    NotFound,
    /// The request itself is wrong, and the message says how.
    BadRequest(String),
    /// The request was based on something that has since changed — a save
    /// naming a revision that is no longer current. Re-read and redo it.
    Conflict(String),
    /// Refused with a status of its own, and fields beside the message that
    /// say more — the item a duplicate already is, the projects that keep a
    /// file from being deleted.
    Refused {
        /// The status.
        status: StatusCode,
        /// What went wrong, in words.
        message: String,
        /// An object whose fields join `error` in the body.
        detail: Value,
    },
    /// Something on our side failed; the detail went to the log.
    Internal,
}

impl ApiError {
    fn refused(status: StatusCode, message: impl ToString, detail: Value) -> Self {
        Self::Refused {
            status,
            message: message.to_string(),
            detail,
        }
    }
}

impl From<LibraryError> for ApiError {
    fn from(error: LibraryError) -> Self {
        let none = json!({});
        match &error {
            LibraryError::NotFound => Self::NotFound,
            LibraryError::Duplicate { id, .. } => {
                Self::refused(StatusCode::CONFLICT, &error, json!({ "item": id }))
            }
            LibraryError::InUse { projects } => {
                Self::refused(StatusCode::CONFLICT, &error, json!({ "projects": projects }))
            }
            // tus answers a chunk that starts in the wrong place with 409, and
            // a client re-asks where it got to.
            LibraryError::Offset { expected } => {
                Self::refused(StatusCode::CONFLICT, &error, json!({ "offset": expected }))
            }
            LibraryError::Busy => Self::refused(StatusCode::LOCKED, &error, none),
            LibraryError::Unsupported(_) => {
                Self::refused(StatusCode::UNSUPPORTED_MEDIA_TYPE, &error, none)
            }
            LibraryError::Rejected(_) => {
                Self::refused(StatusCode::UNPROCESSABLE_ENTITY, &error, none)
            }
            LibraryError::Invalid(_) => Self::BadRequest(error.to_string()),
            LibraryError::Io(_) | LibraryError::Tools(_) | LibraryError::Database(_) => {
                eprintln!("scorsese-server: {error}");
                Self::Internal
            }
        }
    }
}

impl From<AccountError> for ApiError {
    fn from(error: AccountError) -> Self {
        match error {
            AccountError::InvalidEmail(_)
            | AccountError::Taken(_)
            | AccountError::NoSuchAccount(_)
            | AccountError::WeakPassword
            | AccountError::WrongPassword
            | AccountError::UnnamedToken => Self::BadRequest(error.to_string()),
            AccountError::Crypto(_) | AccountError::Files { .. } | AccountError::Database(_) => {
                eprintln!("scorsese-server: {error}");
                Self::Internal
            }
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        eprintln!("scorsese-server: {error}");
        Self::Internal
    }
}

impl From<ProjectError> for ApiError {
    fn from(error: ProjectError) -> Self {
        match error {
            ProjectError::NotFound => Self::NotFound,
            ProjectError::Conflict { .. } => Self::Conflict(error.to_string()),
            ProjectError::Serialize(_) | ProjectError::UnknownFiles { .. } => {
                Self::BadRequest(error.to_string())
            }
            ProjectError::Unreadable { .. }
            | ProjectError::Migrate { .. }
            | ProjectError::Database(_) => {
                eprintln!("scorsese-server: {error}");
                Self::Internal
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::Unauthorized(why) => (StatusCode::UNAUTHORIZED, why.to_owned()),
            Self::Forbidden(why) => (StatusCode::FORBIDDEN, why.to_owned()),
            Self::NotFound => (StatusCode::NOT_FOUND, "not found".to_owned()),
            Self::BadRequest(why) => (StatusCode::BAD_REQUEST, why),
            Self::Conflict(why) => (StatusCode::CONFLICT, why),
            Self::Refused {
                status,
                message,
                mut detail,
            } => {
                if let Some(fields) = detail.as_object_mut() {
                    fields.insert("error".to_owned(), Value::String(message));
                }
                return (status, Json(detail)).into_response();
            }
            Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "something went wrong on the server".to_owned(),
            ),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
