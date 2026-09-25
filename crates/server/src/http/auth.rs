//! Who is asking: the extractor every per-user handler takes.
//!
//! A handler that acts for a user takes a [`Member`] argument, and axum runs
//! this before the handler body — so a handler cannot reach a user's data
//! without one, and a request without valid credentials is answered `401`
//! before any of its code runs.
//!
//! Two ways in, checked in this order:
//!
//! 1. **`Authorization: Bearer scor_…`** — an API token, for anything that is
//!    not a browser. If the header is present it is the answer: a bad token is
//!    a `401`, never a fall-through to a cookie the same client might also
//!    carry, because a request that says who it is and is wrong should not
//!    quietly succeed as somebody else.
//! 2. **The session cookie**, for the browser.
//!
//! ## The cookie
//!
//! `HttpOnly` (no script reads it, so an XSS cannot carry it off), `Secure`
//! (Cloudflare terminates HTTPS in front of us, and browsers treat
//! `localhost` as secure, so development works too), `SameSite=Strict` and
//! `Path=/api`. The web app and the API share one origin, so Strict costs
//! nothing and means no other site can make a browser send it — which, with
//! every write taking a JSON body a plain form cannot produce, is the CSRF
//! defence.

use axum::extract::FromRequestParts;
use axum::http::HeaderMap;
use axum::http::header::{AUTHORIZATION, COOKIE};
use axum::http::request::Parts;

use super::AppState;
use super::error::ApiError;
use crate::accounts::{sessions, tokens};
use crate::db::UserId;

/// The session cookie's name.
pub const SESSION_COOKIE: &str = "scorsese_session";

/// An authenticated user, and how they proved it.
#[derive(Debug, Clone)]
pub struct Member {
    /// Who.
    pub user: UserId,
    /// With what.
    pub via: Via,
}

/// The credential a request was authenticated with.
#[derive(Debug, Clone)]
pub enum Via {
    /// A browser session; the cookie's value, so logging out can end it.
    Session(String),
    /// An API token.
    Token,
}

const NOT_LOGGED_IN: &str = "not logged in";

impl FromRequestParts<AppState> for Member {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        if let Some(header) = parts.headers.get(AUTHORIZATION) {
            let bearer = header
                .to_str()
                .ok()
                .and_then(|value| value.strip_prefix("Bearer "))
                .map(str::trim)
                .ok_or(ApiError::Unauthorized(
                    "the Authorization header is not `Bearer <token>`",
                ))?;
            let user = tokens::find(&state.pool, bearer)
                .await?
                .ok_or(ApiError::Unauthorized("that API token is not valid"))?;
            return Ok(Self {
                user,
                via: Via::Token,
            });
        }
        let cookie = session_cookie(&parts.headers).ok_or(ApiError::Unauthorized(NOT_LOGGED_IN))?;
        let user = sessions::find(&state.pool, &cookie)
            .await?
            .ok_or(ApiError::Unauthorized(NOT_LOGGED_IN))?;
        Ok(Self {
            user,
            via: Via::Session(cookie),
        })
    }
}

/// The session cookie's value, if the request carries one.
pub fn session_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|header| header.to_str().ok())
        .flat_map(|header| header.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == SESSION_COOKIE)
        .map(|(_, value)| value.to_owned())
}

/// The `Set-Cookie` value that logs a browser in with `value`.
pub fn set_cookie(value: &str) -> String {
    let max_age = sessions::LIFETIME_DAYS * 24 * 60 * 60;
    format!("{SESSION_COOKIE}={value}; {ATTRIBUTES}; Max-Age={max_age}")
}

/// The `Set-Cookie` value that makes a browser forget its session.
pub fn clear_cookie() -> String {
    format!("{SESSION_COOKIE}=; {ATTRIBUTES}; Max-Age=0")
}

const ATTRIBUTES: &str = "Path=/api; HttpOnly; Secure; SameSite=Strict";
