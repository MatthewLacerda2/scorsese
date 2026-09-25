//! What a request that did not succeed is answered with.
//!
//! Always `{"error": "…"}` with a status, so the web app and a script read one
//! shape. A failure that is the server's own — the database, the OS — is
//! logged in full and answered with a generic `500`: its detail is for the
//! operator, not for whoever sent the request.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::accounts::AccountError;

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
    /// Something on our side failed; the detail went to the log.
    Internal,
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

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::Unauthorized(why) => (StatusCode::UNAUTHORIZED, why.to_owned()),
            Self::Forbidden(why) => (StatusCode::FORBIDDEN, why.to_owned()),
            Self::NotFound => (StatusCode::NOT_FOUND, "not found".to_owned()),
            Self::BadRequest(why) => (StatusCode::BAD_REQUEST, why),
            Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "something went wrong on the server".to_owned(),
            ),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
