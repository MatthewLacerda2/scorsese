//! Web MCP (#539): `POST /api/mcp`, the Model Context Protocol's Streamable
//! HTTP transport, for a user's own Claude, Gemini CLI or any MCP client.
//!
//! **What a message means is `scorsese_mcp::protocol`'s**, the same code the
//! stdio server answers with, so the handshake, the list and every refusal
//! read the same whichever way a client came in; a `tools/call` is run by
//! [`Toolbox`](crate::tools::Toolbox) for the user the request is from. This
//! module is only the HTTP half.
//!
//! ## The subset of Streamable HTTP served, and why it is enough
//!
//! - **`POST`, answered with `application/json`.** One JSON-RPC message, or a
//!   batch of them (which the 2025-03-26 revision allows), in; the replies
//!   out. A body of nothing but notifications is `202` with no body. The
//!   specification lets a server answer a `POST` with a stream instead; none of
//!   these tools sends progress mid-call — long work is a job, asked after
//!   with `jobs` — so a stream would only ever carry one message.
//! - **No `GET` stream and no `DELETE`: `405`**, which is the specification's
//!   own answer from a server that offers no server-initiated messages and no
//!   sessions. Every tool is told which project it works on, exactly as over
//!   stdio, so there is no session to hold (`docs/mcp.md`, *Stateless, on
//!   purpose*).
//! - **`MCP-Protocol-Version`**, when sent, must be a revision this server
//!   speaks, or the request is `400`.
//!
//! ## Who: an API token, and nothing else
//!
//! `Authorization: Bearer scor_…` (#533) — the token names the user, and
//! every call acts for them alone. A browser session is **refused** here
//! (`403`): a client of this endpoint is a program holding a token, and
//! refusing ambient cookies is also what the specification's warning about
//! DNS rebinding comes down to — a page in somebody's browser cannot borrow
//! their login to reach it. The OAuth flow the specification describes for
//! remote servers is #533's later issue, and ends by issuing these tokens.
//!
//! ## How often: a limit per user
//!
//! [`Limits`]: at most so many tool calls a minute per user, the rest `429`
//! with `Retry-After`. Not a price control — spending already needs a quote
//! and a yes, and a balance — but a runaway client's loop must not fill the
//! render queue or the log for everybody on a shared machine. In memory: it
//! describes this process's last minute, which a restart may forget.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::header::{ALLOW, RETRY_AFTER};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use scorsese_mcp::protocol::{self, Handled, PROTOCOLS};
use serde_json::{Value, json};

use super::AppState;
use super::auth::{Member, Via};
use super::error::ApiError;
use crate::db::UserId;
use crate::tools::Client;

/// The header a client names its protocol revision in, after the handshake.
const VERSION_HEADER: &str = "mcp-protocol-version";

/// Tool calls a user may make in a minute, by default.
pub const CALLS_PER_MINUTE: u32 = 120;

/// `POST /api/mcp`: one message or a batch, answered.
pub async fn post(
    State(state): State<AppState>,
    member: Member,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    if !matches!(member.via, Via::Token) {
        return Err(ApiError::Forbidden(
            "web MCP takes an API token (Authorization: Bearer scor_…), not a browser session",
        ));
    }
    if let Some(version) = headers.get(VERSION_HEADER) {
        let version = version.to_str().unwrap_or_default();
        if !PROTOCOLS.contains(&version) {
            return Err(ApiError::BadRequest(format!(
                "this server does not speak MCP {version}; it speaks {}",
                PROTOCOLS.join(", ")
            )));
        }
    }
    let message: Value = match serde_json::from_slice(&body) {
        Ok(message) => message,
        Err(problem) => {
            return Ok(
                (StatusCode::BAD_REQUEST, Json(protocol::unreadable(problem))).into_response(),
            );
        }
    };
    let (batch, messages) = match message {
        Value::Array(messages) => (true, messages),
        single => (false, vec![single]),
    };
    let calls = messages
        .iter()
        .filter(|message| message.get("method") == Some(&json!("tools/call")))
        .count();
    if let Err(wait) = state.limits.take(member.user, calls) {
        let mut refused = (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({ "error": format!(
                "too many tool calls this minute; try again in {} seconds",
                wait.as_secs().max(1)
            ) })),
        )
            .into_response();
        if let Ok(value) = HeaderValue::from_str(&wait.as_secs().max(1).to_string()) {
            refused.headers_mut().insert(RETRY_AFTER, value);
        }
        return Ok(refused);
    }

    let mut replies = Vec::new();
    for message in messages {
        if let Some(reply) = answer(&state, member.user, message).await {
            replies.push(reply);
        }
    }
    Ok(match (batch, replies.len()) {
        (_, 0) => StatusCode::ACCEPTED.into_response(),
        (false, _) => Json(replies.swap_remove(0)).into_response(),
        (true, _) => Json(Value::Array(replies)).into_response(),
    })
}

/// `GET` and `DELETE /api/mcp`: `405`. No server-initiated stream is offered,
/// and there are no sessions to end.
pub async fn refuse() -> Response {
    let mut response = (
        StatusCode::METHOD_NOT_ALLOWED,
        Json(json!({ "error": "this endpoint takes POST only: no stream, no sessions" })),
    )
        .into_response();
    response
        .headers_mut()
        .insert(ALLOW, HeaderValue::from_static("POST"));
    response
}

/// One message's reply, or `None` for a notification.
async fn answer(state: &AppState, user: UserId, message: Value) -> Option<Value> {
    match protocol::handle(message, || state.tools.listing()) {
        Handled::Silent => None,
        Handled::Answered(reply) => Some(reply),
        Handled::Call(call) if !state.tools.serves(&call.name) => Some(call.unknown()),
        Handled::Call(call) => {
            let outcome = state
                .tools
                .call(user, Client::External, &call.name, &call.arguments)
                .await;
            Some(call.answer(outcome))
        }
    }
}

/// How many tool calls each user has made in the current minute.
#[derive(Debug, Clone)]
pub struct Limits {
    per_minute: u32,
    windows: Arc<Mutex<HashMap<UserId, (Instant, u32)>>>,
}

impl Default for Limits {
    fn default() -> Self {
        Self::new(CALLS_PER_MINUTE)
    }
}

impl Limits {
    /// At most `per_minute` tool calls a minute for each user.
    pub fn new(per_minute: u32) -> Self {
        Self {
            per_minute,
            windows: Arc::default(),
        }
    }

    /// Count `calls` more for `user`, or say how long until they may.
    pub(crate) fn take(&self, user: UserId, calls: usize) -> Result<(), Duration> {
        if calls == 0 {
            return Ok(());
        }
        let calls = u32::try_from(calls).unwrap_or(u32::MAX);
        let now = Instant::now();
        let mut windows = self.windows.lock().unwrap_or_else(PoisonError::into_inner);
        let window = windows.entry(user).or_insert((now, 0));
        if now.duration_since(window.0) >= MINUTE {
            *window = (now, 0);
        }
        if window.1.saturating_add(calls) > self.per_minute {
            return Err(MINUTE.saturating_sub(now.duration_since(window.0)));
        }
        window.1 += calls;
        Ok(())
    }
}

/// The window a limit counts over.
const MINUTE: Duration = Duration::from_secs(60);
