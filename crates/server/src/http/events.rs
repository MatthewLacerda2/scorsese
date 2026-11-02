//! `GET /api/events`: everything live for the caller, as server-sent events.
//! What is on it, and why SSE, is [`crate::events`].

use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{self, KeepAlive, Sse};
use futures_util::{Stream, StreamExt, future};
use sqlx::postgres::PgPool;
use tokio::sync::oneshot;

use super::AppState;
use super::auth::{Member, Via};
use crate::accounts::{AccountError, sessions, tokens};
use crate::db::UserId;

/// How often an open stream asks whether its credential still holds.
pub const RECHECK: Duration = Duration::from_secs(15);

/// How many streams one user may hold open; one more closes their oldest.
pub const MOST_STREAMS: usize = 8;

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
///
/// One user holds at most [`MOST_STREAMS`] at once ([`Streams`]): opening
/// one more ends their oldest, the tab they are least likely to be looking at.
pub async fn stream(
    State(state): State<AppState>,
    member: Member,
) -> Sse<impl Stream<Item = Result<sse::Event, Infallible>>> {
    let events = state.events.subscribe(member.user);
    let (held, displaced) = state.streams.open(member.user);
    let lapsed = Box::pin(lapsed(state.pool, state.recheck, member));
    let ended = future::select(lapsed, displaced);
    let stream = events.take_until(ended).map(move |event| {
        let _held = &held;
        // An enum of plain data serialises; there is no error to report.
        let data = serde_json::to_string(&event).unwrap_or_default();
        Ok(sse::Event::default().data(data))
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Done once `member`'s credential no longer holds, asked every `recheck`.
///
/// A lookup that fails is asked again at the next one, and never ends the
/// stream: a reconnect during the same outage would be answered `500` by the
/// extractor, and an `EventSource` answered anything but `200` gives up for
/// good, leaving the page without live updates until it is reloaded.
async fn lapsed(pool: PgPool, recheck: Duration, member: Member) {
    loop {
        tokio::time::sleep(recheck).await;
        match holds(&pool, &member).await {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => tracing::warn!("rechecking a live stream's credential: {error}"),
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

/// The event streams each user holds open, oldest first, each with the
/// sender whose dropping ends it. In memory: a restart ends every stream.
#[derive(Debug, Clone, Default)]
pub struct Streams(Arc<Mutex<Open>>);

#[derive(Debug, Default)]
struct Open {
    next: u64,
    by_user: HashMap<UserId, VecDeque<(u64, oneshot::Sender<()>)>>,
}

/// One open stream's place in [`Streams`]; letting go gives it up.
#[derive(Debug)]
pub struct Held {
    streams: Streams,
    user: UserId,
    id: u64,
}

impl Streams {
    /// Make room for one more of `user`'s streams, ending their oldest past
    /// [`MOST_STREAMS`]: its place, and what resolves when it is displaced.
    pub fn open(&self, user: UserId) -> (Held, oneshot::Receiver<()>) {
        let (displace, displaced) = oneshot::channel();
        let mut open = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let id = open.next;
        open.next += 1;
        let held = open.by_user.entry(user).or_default();
        while held.len() >= MOST_STREAMS {
            // Dropping its sender ends that stream.
            held.pop_front();
        }
        held.push_back((id, displace));
        let held = Held {
            streams: self.clone(),
            user,
            id,
        };
        (held, displaced)
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        let mut open = self
            .streams
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(held) = open.by_user.get_mut(&self.user) {
            held.retain(|(id, _)| *id != self.id);
            if held.is_empty() {
                open.by_user.remove(&self.user);
            }
        }
    }
}
