//! Sign in with Google (#952): the two endpoints a browser passes through.
//!
//! The OAuth 2.0 authorization-code flow with PKCE, done server-side, as
//! OpenID Connect describes it:
//!
//! 1. **`GET /api/auth/google`** — the "Continue with Google" button is a link
//!    here. It mints a [`Flow`] (a `state`, a PKCE verifier, where to land),
//!    hands it to the browser as a ten-minute cookie, and sends the browser to
//!    Google with the state and the verifier's challenge.
//! 2. **`GET /api/auth/google/callback`** — Google sends the browser back with
//!    a code and the state. The state must match the cookie's; the code is
//!    traded for an ID token back-channel, with the verifier and the client's
//!    secret ([`scorsese_providers::sign_in`] — which also says why the
//!    token's signature is not checked); the token names a Google account,
//!    which [`users::sign_in`] turns into a scorsese account; and the browser
//!    gets **the same session cookie a login always gave it** ([`auth`]) and
//!    is sent on to where it was going, `/projects` by default.
//!
//! Every way that can fail ends on `/login?error=<why>`, where the web app
//! says it in words: `unconfigured`, `cancelled` (the person said no at
//! Google), `expired` (no matching flow cookie: too slow, or another tab),
//! `unverified`, `linked` (the email's account belongs to another Google
//! account) and `failed` (anything else, which the server's log names). A
//! redirect rather than JSON because a person is looking at it.

pub mod flow;

use std::net::IpAddr;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::header::{LOCATION, SET_COOKIE};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{AppendHeaders, IntoResponse, Response};
use scorsese_providers::credentials::{Environment, Secret};
use scorsese_providers::sign_in::{self, Client, SignInError, TokenEndpoint};
use serde::Deserialize;

pub use flow::Flow;

use super::AppState;
use super::auth;
use super::visitor::Visitor;
use crate::accounts::access::{self, Access};
use crate::accounts::{AccountError, sessions, users};
use crate::config::{ConfigError, GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET, PUBLIC_ORIGIN};

/// Where Google sends a browser back to, under the public origin.
pub const CALLBACK: &str = "/api/auth/google/callback";

/// The server's Google OAuth client, and where it is reached.
#[derive(Clone)]
pub struct Google {
    /// The client registered with Google.
    pub client: Client,
    /// The site's origin, with no trailing slash.
    pub origin: String,
    /// Where codes are traded: Google, or a test's recording.
    pub endpoint: Arc<dyn TokenEndpoint>,
}

impl std::fmt::Debug for Google {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Google")
            .field("client", &self.client)
            .field("origin", &self.origin)
            .finish_non_exhaustive()
    }
}

impl Google {
    /// The client the environment names, if it names one whole.
    ///
    /// All three variables or none: one missing is a server without Google
    /// sign-in, which still serves everything else ([`super::super::Config`]).
    pub fn from_environment(environment: &Environment) -> Result<Option<Self>, ConfigError> {
        let (Some(id), Some(secret), Some(origin)) = (
            environment.get(GOOGLE_CLIENT_ID),
            environment.get(GOOGLE_CLIENT_SECRET),
            environment.get(PUBLIC_ORIGIN),
        ) else {
            return Ok(None);
        };
        let origin = public_origin(origin).ok_or_else(|| ConfigError::Origin {
            value: origin.to_owned(),
        })?;
        Ok(Some(Self {
            client: Client {
                id: id.trim().to_owned(),
                secret: Secret::new(secret.trim()),
            },
            origin,
            endpoint: Arc::new(sign_in::Google),
        }))
    }

    /// The same client, trading codes at `endpoint` — a test's.
    pub fn trading_at(mut self, endpoint: Arc<dyn TokenEndpoint>) -> Self {
        self.endpoint = endpoint;
        self
    }

    /// The redirect URI registered on the client.
    pub fn redirect_uri(&self) -> String {
        format!("{}{CALLBACK}", self.origin)
    }
}

/// `value` as an origin a browser can be sent back to: `https://` and a host,
/// or plain `http://` on this machine alone, where browsers allow a `Secure`
/// cookie anyway. No path, query or fragment; a trailing slash is dropped.
pub(crate) fn public_origin(value: &str) -> Option<String> {
    let origin = value.trim().trim_end_matches('/');
    let local = |host: &&str| {
        ["localhost", "127.0.0.1"].iter().any(|name| {
            *host == *name
                || host
                    .strip_prefix(name)
                    .is_some_and(|port| port.starts_with(':'))
        })
    };
    let host = origin
        .strip_prefix("https://")
        .or_else(|| origin.strip_prefix("http://").filter(local))?;
    (!host.is_empty() && !host.contains(['/', '?', '#', '@'])).then(|| origin.to_owned())
}

/// What a sign-in may be asked to land on.
#[derive(Debug, Deserialize)]
pub struct Start {
    /// A path on this site.
    pub next: Option<String>,
}

/// `GET /api/auth/google`: start a sign-in.
pub async fn start(State(state): State<AppState>, Query(start): Query<Start>) -> Response {
    let Some(google) = &state.google else {
        return to_login("unconfigured");
    };
    let flow = match Flow::mint(start.next.as_deref()) {
        Ok(flow) => flow,
        Err(error) => return failed(&error),
    };
    let url = sign_in::authorize_url(
        &google.client,
        &google.redirect_uri(),
        &flow.state,
        &flow.challenge(),
    );
    (
        StatusCode::SEE_OTHER,
        AppendHeaders([(LOCATION, url), (SET_COOKIE, flow.set_cookie())]),
    )
        .into_response()
}

/// What Google sends the browser back with.
#[derive(Debug, Deserialize)]
pub struct Callback {
    /// The one-time code, on success.
    pub code: Option<String>,
    /// The state the sign-in was started with.
    pub state: Option<String>,
    /// Why there is no code: `access_denied` when the person said no.
    pub error: Option<String>,
}

/// `GET /api/auth/google/callback`: finish a sign-in.
pub async fn callback(
    State(state): State<AppState>,
    Visitor(address): Visitor,
    headers: HeaderMap,
    Query(callback): Query<Callback>,
) -> Response {
    let Some(google) = state.google.clone() else {
        return to_login("unconfigured");
    };
    let flow = Flow::from_headers(&headers);
    let signed = finish(&state, google, flow.as_ref(), callback, address).await;
    let clear = (SET_COOKIE, flow::clear_cookie());
    match signed {
        Ok(cookie) => {
            let next = flow.map_or_else(|| flow::HOME.to_owned(), |flow| flow.next);
            (
                StatusCode::SEE_OTHER,
                AppendHeaders([
                    (LOCATION, next),
                    (SET_COOKIE, auth::set_cookie(&cookie)),
                    clear,
                ]),
            )
                .into_response()
        }
        Err(why) => (
            StatusCode::SEE_OTHER,
            AppendHeaders([(LOCATION, format!("/login?error={why}")), clear]),
        )
            .into_response(),
    }
}

/// The session cookie this callback earns, or the `error` it ends on.
async fn finish(
    state: &AppState,
    google: Google,
    flow: Option<&Flow>,
    callback: Callback,
    address: Option<IpAddr>,
) -> Result<String, &'static str> {
    if callback.error.is_some() {
        return Err("cancelled");
    }
    let (Some(flow), Some(code), Some(returned)) = (flow, callback.code, callback.state) else {
        return Err("expired");
    };
    if returned != flow.state {
        return Err("expired");
    }
    let verifier = flow.verifier.clone();
    let traded = tokio::task::spawn_blocking(move || {
        let redirect_uri = google.redirect_uri();
        let form = sign_in::exchange_form(&google.client, &redirect_uri, &code, &verifier);
        let id_token = google.endpoint.exchange(&form)?;
        sign_in::identity(&id_token, &google.client.id, now())
    })
    .await
    .map_err(|_| "failed")?;
    let identity = traded.map_err(|error| match error {
        SignInError::Unverified => "unverified",
        error => {
            eprintln!("scorsese-server: a Google sign-in failed: {error}");
            "failed"
        }
    })?;
    let signed = users::sign_in(&state.pool, &identity)
        .await
        .map_err(|error| match error {
            AccountError::LinkedElsewhere(_) => "linked",
            error => {
                eprintln!("scorsese-server: a Google sign-in failed: {error}");
                "failed"
            }
        })?;
    if let users::SignedIn::Created(user) = signed {
        eprintln!(
            "scorsese-server: account {} created by a Google sign-in",
            user.get()
        );
    }
    let cookie = sessions::open(&state.pool, signed.user())
        .await
        .map_err(|error| {
            eprintln!("scorsese-server: a Google sign-in failed: {error}");
            "failed"
        })?;
    access::record_or_log(&state.pool, signed.user(), address, Access::SignIn).await;
    Ok(cookie)
}

/// Seconds since the epoch, for an ID token's `exp`.
fn now() -> i64 {
    scorsese_core::Timestamp::unix_now().unwrap_or(0)
}

/// Back to the login page, saying `why`.
fn to_login(why: &str) -> Response {
    (
        StatusCode::SEE_OTHER,
        [(LOCATION, format!("/login?error={why}"))],
    )
        .into_response()
}

/// A failure of our own, before Google was involved.
fn failed(error: &AccountError) -> Response {
    eprintln!("scorsese-server: a Google sign-in could not start: {error}");
    to_login("failed")
}
