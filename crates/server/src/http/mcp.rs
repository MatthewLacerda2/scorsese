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
//! ## Stopping a call: `notifications/cancelled`
//!
//! A client that no longer wants a call's answer says so in a notification
//! naming the call's id — over this transport in a `POST` of its own, while
//! the call's `POST` is still waiting. So the calls in flight are kept per
//! signed-in user ([`InFlight`]), not per connection: a cancel trips the
//! [`Cancel`] of that user's call with that id, and nobody else's. The tool is
//! handed it as the stdio server hands it over (`Tool::call_cancellable`,
//! #647), and a cancelled call is not answered, as the specification asks. A
//! client that hangs up mid-call has stopped wanting it too, and its call is
//! cancelled the same way.
//!
//! A `render` is not stopped this way here: its call answers the moment the
//! job is queued, so there is no call left to cancel by the time anybody
//! wants to. Stopping the render itself is `job_cancel` (#660).
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
use axum::http::header::ALLOW;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use scorsese_mcp::protocol::{self, Handled, PROTOCOLS};
use scorsese_render::Cancel;
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
        let retry_after = wait.as_secs().max(1);
        return Err(ApiError::TooMany {
            message: format!("too many tool calls this minute; try again in {retry_after} seconds"),
            retry_after,
        });
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

/// One message's reply, or `None` for a notification or a cancelled call.
async fn answer(state: &AppState, user: UserId, message: Value) -> Option<Value> {
    if message.get("method") == Some(&json!("notifications/cancelled"))
        && let Some(id) = message.pointer("/params/requestId")
    {
        state.in_flight.cancel(user, id);
    }
    let id = message.get("id").cloned().unwrap_or(Value::Null);
    match protocol::handle(message, || state.tools.listing()) {
        Handled::Silent => None,
        Handled::Answered(reply) => Some(reply),
        Handled::Call(call) if !state.tools.serves(&call.name) => Some(call.unknown()),
        Handled::Call(call) => {
            let flight = state.in_flight.begin(user, &id);
            let outcome = state
                .tools
                .call_cancellable(
                    user,
                    Client::External,
                    &call.name,
                    &call.arguments,
                    &flight.cancel,
                )
                .await;
            if flight.cancel.is_cancelled() {
                return None;
            }
            Some(call.answer(outcome))
        }
    }
}

/// The web MCP calls each user has running, by the id their client gave it —
/// written as JSON, as the stdio server keys them, since `1` and `"1"` are
/// different ids. Cheap to clone.
#[derive(Debug, Clone, Default)]
pub struct InFlight {
    calls: Arc<Mutex<HashMap<(UserId, String), Cancel>>>,
}

impl InFlight {
    /// Register `user`'s call `id`, until the returned guard is dropped.
    fn begin(&self, user: UserId, id: &Value) -> Flight {
        let key = (user, id.to_string());
        let cancel = Cancel::new();
        self.lock().insert(key.clone(), cancel.clone());
        Flight {
            calls: self.clone(),
            key,
            cancel,
        }
    }

    /// Stop `user`'s call `id`, if it is running. Another user's call with
    /// the same id is not theirs to stop, and is not found.
    pub(crate) fn cancel(&self, user: UserId, id: &Value) {
        if let Some(cancel) = self.lock().get(&(user, id.to_string())) {
            cancel.cancel();
        }
    }

    /// No code under the lock can panic, so a poisoned map is used as it is.
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<(UserId, String), Cancel>> {
        self.calls.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// One call in flight. Dropped when it is answered — or when the request is
/// dropped because the client hung up, which trips its cancel: nobody is
/// left to read the answer.
struct Flight {
    calls: InFlight,
    key: (UserId, String),
    cancel: Cancel,
}

impl Drop for Flight {
    fn drop(&mut self) {
        self.calls.lock().remove(&self.key);
        self.cancel.cancel();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cancel_stops_only_its_own_users_call_with_that_exact_id() {
        let (ana, bob) = (UserId::from_row(1), UserId::from_row(2));
        let in_flight = InFlight::default();
        let call = in_flight.begin(ana, &json!(1));

        in_flight.cancel(bob, &json!(1));
        in_flight.cancel(ana, &json!("1"));
        assert!(
            !call.cancel.is_cancelled(),
            "bob's id, or a string id, is another call"
        );
        in_flight.cancel(ana, &json!(1));
        assert!(call.cancel.is_cancelled());
    }

    #[test]
    fn a_call_is_forgotten_once_answered_and_stopped_if_its_client_hung_up() {
        let ana = UserId::from_row(1);
        let in_flight = InFlight::default();
        let cancel = in_flight.begin(ana, &json!(7)).cancel.clone();
        assert!(
            cancel.is_cancelled(),
            "dropped mid-call: nobody reads the answer"
        );
        assert!(in_flight.lock().is_empty());
    }
}
