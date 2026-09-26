//! `GET /api/events`: everything live for the caller, as server-sent events.
//! What is on it, and why SSE, is [`crate::events`].

use std::convert::Infallible;

use axum::extract::State;
use axum::response::sse::{self, KeepAlive, Sse};
use futures_util::{Stream, StreamExt};

use super::AppState;
use super::auth::Member;

/// The caller's live stream. Each message's data is one JSON
/// [`Event`](crate::events::Event) with a `type`. A comment every fifteen
/// seconds keeps a quiet stream from being closed as idle by what sits in
/// between — nginx, the tunnel. It ends when the server stops, and a browser's
/// `EventSource` reconnects by itself.
pub async fn stream(
    State(state): State<AppState>,
    member: Member,
) -> Sse<impl Stream<Item = Result<sse::Event, Infallible>>> {
    let stream = state.events.subscribe(member.user).map(|event| {
        // An enum of plain data serialises; there is no error to report.
        let data = serde_json::to_string(&event).unwrap_or_default();
        Ok(sse::Event::default().data(data))
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}
