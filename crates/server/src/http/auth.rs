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
//! Either way, an account's first request of the day is written to the access
//! record (`accounts::access`, #1267) on its way through.
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
//!
//! `Secure` is unconditional, and that is a decision (#568): a browser on
//! `http://` from another device cannot stay logged in, and there is
//! deliberately no setting that drops the attribute for a "trusted network".
//! Every way in before there is a domain — a quick tunnel, `tailscale serve` —
//! is HTTPS already; `docs/web.md`, *Testing before there is a domain*, has
//! them and the argument.
//!
//! ## The hint beside it
//!
//! Because no script can read the session cookie, the web app cannot tell on
//! first paint whether `/` should be the landing page or the projects list
//! without a round trip to `/api/me`. So every response that sets or clears
//! the session sets or clears [`HINT_COOKIE`] too (#1246): readable, `Path=/`,
//! the same lifetime, and carrying nothing — no id, no token, only `1`. It
//! decides what is drawn while `/api/me` is asked, never who anybody is: a
//! forged one costs its owner the wait a signed-in visitor has, and a stale one
//! is cleared by the page when `/api/me` answers `401`.

use axum::extract::FromRequestParts;
use axum::http::HeaderMap;
use axum::http::header::{AUTHORIZATION, COOKIE};
use axum::http::request::Parts;

use super::AppState;
use super::error::ApiError;
use super::visitor::Visitor;
use crate::accounts::{sessions, tokens};
use crate::db::UserId;

/// The session cookie's name.
pub const SESSION_COOKIE: &str = "scorsese_session";

/// The script-readable "a session was set here" hint's name (`web/src/session/hint.ts`).
pub const HINT_COOKIE: &str = "scorsese_signed_in";

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
    /// An API token, by its id.
    Token(i64),
}

const NOT_LOGGED_IN: &str = "not logged in";

impl FromRequestParts<AppState> for Member {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let member = authenticate(parts, state).await?;
        crate::logs::signed_in(member.user);
        let Visitor(address) = Visitor::of(parts);
        state.daily.note(&state.pool, member.user, address).await;
        Ok(member)
    }
}

/// Who `parts` says they are, by token or by cookie.
async fn authenticate(parts: &Parts, state: &AppState) -> Result<Member, ApiError> {
    if let Some(header) = parts.headers.get(AUTHORIZATION) {
        let bearer = header
            .to_str()
            .ok()
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(str::trim)
            .ok_or(ApiError::Unauthorized(
                "the Authorization header is not `Bearer <token>`",
            ))?;
        let (user, token) = tokens::find(&state.pool, bearer)
            .await?
            .ok_or(ApiError::Unauthorized("that API token is not valid"))?;
        return Ok(Member {
            user,
            via: Via::Token(token),
        });
    }
    let cookie = session_cookie(&parts.headers).ok_or(ApiError::Unauthorized(NOT_LOGGED_IN))?;
    let user = sessions::find(&state.pool, &cookie)
        .await?
        .ok_or(ApiError::Unauthorized(NOT_LOGGED_IN))?;
    Ok(Member {
        user,
        via: Via::Session(cookie),
    })
}

/// The session cookie's value, if the request carries one.
pub fn session_cookie(headers: &HeaderMap) -> Option<String> {
    cookie(headers, SESSION_COOKIE).map(str::to_owned)
}

/// The value of the cookie called `name`, if the request carries one: the
/// first, across every `Cookie` header.
pub fn cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|header| header.to_str().ok())
        .flat_map(|header| header.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(found, _)| *found == name)
        .map(|(_, value)| value)
}

/// The `Set-Cookie` values that log a browser in with `value`: the session,
/// and the hint that one is there.
pub fn set_cookies(value: &str) -> [String; 2] {
    let max_age = sessions::LIFETIME_DAYS * 24 * 60 * 60;
    [
        format!("{SESSION_COOKIE}={value}; {ATTRIBUTES}; Max-Age={max_age}"),
        format!("{HINT_COOKIE}=1; {HINT_ATTRIBUTES}; Max-Age={max_age}"),
    ]
}

/// The `Set-Cookie` values that make a browser forget its session, and the hint.
pub fn clear_cookies() -> [String; 2] {
    [
        format!("{SESSION_COOKIE}=; {ATTRIBUTES}; Max-Age=0"),
        format!("{HINT_COOKIE}=; {HINT_ATTRIBUTES}; Max-Age=0"),
    ]
}

const ATTRIBUTES: &str = "Path=/api; HttpOnly; Secure; SameSite=Strict";

/// The session's attributes but two: every page reads it, and so does script.
const HINT_ATTRIBUTES: &str = "Path=/; Secure; SameSite=Strict";

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn headers(values: &[&str]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for value in values {
            headers.append(COOKIE, HeaderValue::from_str(value).unwrap());
        }
        headers
    }

    #[test]
    fn a_cookie_is_found_among_others_in_any_header() {
        let headers = headers(&["a=1; scorsese_session=abc", "b=2;flow=x.y"]);
        assert_eq!(cookie(&headers, SESSION_COOKIE), Some("abc"));
        assert_eq!(cookie(&headers, "flow"), Some("x.y"));
        assert_eq!(cookie(&headers, "b"), Some("2"));
        assert_eq!(session_cookie(&headers).as_deref(), Some("abc"));
    }

    #[test]
    fn a_missing_or_valueless_cookie_is_none() {
        let headers = headers(&["a=1; scorsese_session", "other"]);
        assert_eq!(cookie(&headers, SESSION_COOKIE), None);
        assert_eq!(cookie(&headers, "c"), None);
        assert_eq!(cookie(&HeaderMap::new(), "a"), None);
    }

    #[test]
    fn the_first_of_two_with_one_name_wins() {
        let headers = headers(&["a=1; a=2"]);
        assert_eq!(cookie(&headers, "a"), Some("1"));
    }
}
