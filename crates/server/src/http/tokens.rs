//! A user's API tokens: list, issue, revoke.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Deserialize;

use super::AppState;
use super::auth::{Member, Via};
use super::error::ApiError;
use crate::accounts::tokens::{self, Issued, TokenInfo};

/// `GET /api/tokens`: the caller's tokens, never their values.
pub async fn list(
    State(state): State<AppState>,
    member: Member,
) -> Result<Json<Vec<TokenInfo>>, ApiError> {
    Ok(Json(tokens::list(&state.pool, member.user).await?))
}

/// `POST /api/tokens`'s body.
#[derive(Debug, Deserialize)]
pub struct NewToken {
    /// What to call it, to tell it apart when revoking.
    pub name: String,
}

/// `POST /api/tokens`: issue a token. Its value is in this response and
/// nowhere else, ever.
///
/// **Only from a browser session.** A token that could issue tokens would
/// make a leaked one impossible to contain by revoking it: whoever held it
/// would already have minted another.
pub async fn issue(
    State(state): State<AppState>,
    member: Member,
    Json(new): Json<NewToken>,
) -> Result<(StatusCode, Json<Issued>), ApiError> {
    if matches!(member.via, Via::Token) {
        return Err(ApiError::Forbidden(
            "an API token cannot issue tokens; log in to the web app to make one",
        ));
    }
    let issued = tokens::issue(&state.pool, member.user, &new.name).await?;
    Ok((StatusCode::CREATED, Json(issued)))
}

/// `DELETE /api/tokens/{id}`: revoke one of the caller's tokens.
///
/// `404` for an id that is not theirs, whether or not it is somebody else's.
pub async fn revoke(
    State(state): State<AppState>,
    member: Member,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    if tokens::revoke(&state.pool, member.user, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}
