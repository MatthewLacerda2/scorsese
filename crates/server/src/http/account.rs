//! Logging out, and the account a request is logged in as. Logging in is
//! Google's ([`super::google`]).

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::SET_COOKIE;
use axum::response::IntoResponse;

use super::AppState;
use super::auth::{self, Member, Via};
use super::error::ApiError;
use crate::accounts::{Account, sessions, users};

/// `POST /api/logout`: end this browser's session, and tell it to forget the
/// cookie.
///
/// With an API token there is no session to end, and the answer is the same
/// `204` — logging out is idempotent, and a token is revoked by name instead.
pub async fn logout(
    State(state): State<AppState>,
    member: Member,
) -> Result<impl IntoResponse, ApiError> {
    if let Via::Session(cookie) = &member.via {
        sessions::close(&state.pool, member.user, cookie).await?;
    }
    Ok((StatusCode::NO_CONTENT, [(SET_COOKIE, auth::clear_cookie())]))
}

/// `GET /api/me`: who this request is logged in as.
pub async fn me(State(state): State<AppState>, member: Member) -> Result<Json<Account>, ApiError> {
    Ok(Json(users::get(&state.pool, member.user).await?))
}
