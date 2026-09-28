//! Logging in and out, and the account a request is logged in as.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::SET_COOKIE;
use axum::response::IntoResponse;
use serde::Deserialize;

use super::AppState;
use super::auth::{self, Member, Via};
use super::client::ClientAddress;
use super::error::ApiError;
use crate::accounts::throttle::{self, Key};
use crate::accounts::{Account, sessions, users};

/// `POST /api/login`'s body.
#[derive(Debug, Deserialize)]
pub struct Login {
    /// The account's email, in any case.
    pub email: String,
    /// Its password.
    pub password: String,
}

/// `POST /api/login`: the account, and a session cookie for it.
///
/// One message for an unknown email and a wrong password alike — which of
/// the two it was is exactly what somebody guessing would like to know.
///
/// Braked first ([`throttle`]): the attempt is counted against the email and
/// the client's address before the password is looked at, and refused with
/// `429` once either has had too many. The refusal is the same for every
/// email, real or not, and comes before argon2 for every email alike.
pub async fn login(
    State(state): State<AppState>,
    ClientAddress(address): ClientAddress,
    Json(login): Json<Login>,
) -> Result<impl IntoResponse, ApiError> {
    let email = Key::email(&login.email);
    let address = address.map(Key::address);
    let keys: Vec<Key> = [Some(email.clone()), address.clone()]
        .into_iter()
        .flatten()
        .collect();
    if let Err(wait) = throttle::admit(&state.pool, &keys).await? {
        let minutes = wait.as_secs().div_ceil(60);
        return Err(ApiError::TooMany {
            message: format!(
                "too many login attempts; try again in {minutes} minute{}",
                if minutes == 1 { "" } else { "s" }
            ),
            retry_after: wait.as_secs(),
        });
    }
    let user = users::authenticate(&state.pool, &login.email, &login.password)
        .await?
        .ok_or(ApiError::Unauthorized(
            "that email and password do not match an account",
        ))?;
    throttle::succeeded(&state.pool, &email, address.as_ref()).await?;
    let cookie = sessions::open(&state.pool, user).await?;
    let account = users::get(&state.pool, user).await?;
    Ok(([(SET_COOKIE, auth::set_cookie(&cookie))], Json(account)))
}

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

/// `POST /api/me/password`'s body.
#[derive(Debug, Deserialize)]
pub struct PasswordChange {
    /// The password now.
    pub current: String,
    /// The password to have instead.
    pub new: String,
}

/// `POST /api/me/password`: change one's own password, knowing the current
/// one. The operator's reset is the way back for somebody who does not.
pub async fn change_password(
    State(state): State<AppState>,
    member: Member,
    Json(change): Json<PasswordChange>,
) -> Result<StatusCode, ApiError> {
    users::change_password(&state.pool, member.user, &change.current, &change.new).await?;
    Ok(StatusCode::NO_CONTENT)
}
