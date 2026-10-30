//! Logging out, and the account a request is logged in as. Logging in is
//! Google's ([`super::google`]).

use axum::Json;
use axum::extract::State;
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderName, StatusCode};
use axum::response::{AppendHeaders, IntoResponse};

/// The header that asks a browser to drop what it stored for this origin.
const CLEAR_SITE_DATA: HeaderName = HeaderName::from_static("clear-site-data");

use super::AppState;
use super::auth::{self, Member, Via};
use super::error::ApiError;
use crate::accounts::{Account, sessions, users};

/// `POST /api/logout`: end this browser's session, and tell it to forget the
/// cookie, its hint and its cache (`Clear-Site-Data: "cache"`).
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
    let [session, hint] = auth::clear_cookies();
    Ok((
        StatusCode::NO_CONTENT,
        // Appended: a plain array of headers keeps only the last of a name.
        AppendHeaders([
            (SET_COOKIE, session),
            (SET_COOKIE, hint),
            // Every private file this browser kept goes too; nothing of the
            // account is left to open from its cache.
            (CLEAR_SITE_DATA, "\"cache\"".to_owned()),
        ]),
    ))
}

/// `GET /api/me`: who this request is logged in as.
pub async fn me(State(state): State<AppState>, member: Member) -> Result<Json<Account>, ApiError> {
    Ok(Json(users::get(&state.pool, member.user).await?))
}
