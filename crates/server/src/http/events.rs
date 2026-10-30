//! `GET /api/events`: everything live for the caller, as server-sent events.
//! What is on it, and why SSE, is [`crate::events`].

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{self, KeepAlive, Sse};
use futures_util::{Stream, StreamExt};
use sqlx::postgres::PgPool;

use super::AppState;
use super::auth::{Member, Via};
use crate::accounts::{AccountError, sessions, tokens};

/// How often an open stream asks whether its credential still holds.
pub const RECHECK: Duration = Duration::from_secs(15);

/// The caller's live stream. Each message's data is one JSON
/// [`Event`](crate::events::Event) with a `type`. A comment every fifteen
/// seconds keeps a quiet stream from being closed as idle by what sits in
/// between — nginx, the tunnel.
///
/// It lasts as long as the credential it was opened with: every
/// [`AppState::recheck`] the session or token is looked up again, and the
/// stream ends once logging out, the operator's `user logout`, a revoked
/// token or an expired session has removed it. It ends when the server stops,
/// too, and a browser's `EventSource` reconnects by itself, through the
/// extractor that refuses a credential gone.
pub async fn stream(
    State(state): State<AppState>,
    member: Member,
) -> Sse<impl Stream<Item = Result<sse::Event, Infallible>>> {
    let events = state.events.subscribe(member.user);
    let lapsed = lapsed(state.pool, state.recheck, member);
    let stream = events.take_until(lapsed).map(|event| {
        // An enum of plain data serialises; there is no error to report.
        let data = serde_json::to_string(&event).unwrap_or_default();
        Ok(sse::Event::default().data(data))
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Done once `member`'s credential no longer holds, asked every `recheck`. A
/// lookup that fails ends it too: a stream the database cannot vouch for is
/// closed, and the reconnect is authenticated afresh.
async fn lapsed(pool: PgPool, recheck: Duration, member: Member) {
    loop {
        tokio::time::sleep(recheck).await;
        match holds(&pool, &member).await {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => {
                tracing::warn!("rechecking a live stream's credential: {error}");
                return;
            }
        }
    }
}

/// Whether the session or token `member` was authenticated with is still
/// theirs.
async fn holds(pool: &PgPool, member: &Member) -> Result<bool, AccountError> {
    match &member.via {
        Via::Session(cookie) => Ok(sessions::find(pool, cookie).await? == Some(member.user)),
        Via::Token(id) => tokens::holds(pool, member.user, *id).await,
    }
}
