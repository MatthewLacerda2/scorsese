//! What the server tells a user's browser as it happens: one stream per
//! user, `GET /api/events` (#536).
//!
//! **One stream for everything live.** Job state was the first thing on it;
//! the assistant (#540) added its turns, words, progress notes, tool calls and
//! quotes — and a project's new revision — as more [`Event`] variants rather
//! than opening a second connection. A browser holds one
//! `EventSource` and switches on each message's `type`.
//!
//! **Server-sent events, not a WebSocket.** Everything here flows one way —
//! a browser *asks* for things over ordinary requests — and SSE is plain HTTP:
//! it passes nginx and the Cloudflare tunnel untouched, reconnects by itself,
//! and carries the session cookie like any other request under `/api`.
//!
//! **In memory, and allowed to drop.** A broadcast channel inside this one
//! process; nothing is stored. A browser that falls too far behind, or
//! reconnects, gets [`Event::Resync`] or nothing at all, and the answer is the
//! same either way: fetch the current state (`GET /api/jobs`, and for a chat
//! `GET /api/projects/{id}/chat`) and carry on from the stream. The database is the record; this is only the nudge.

use std::sync::Arc;

use futures_util::Stream;
use serde::Serialize;
use tokio::sync::{broadcast, watch};

use crate::assistant::{QuoteView, TurnView};
use crate::db::UserId;
use crate::jobs::{JobView, ProgressView};

/// How many events may sit unread before a slow reader is told to resync.
const BACKLOG: usize = 256;

/// One thing a user's browser is told.
///
/// Serialised with a `type` field naming the variant, e.g.
/// `{"type": "job", "id": 7, "state": "running", …}`. **New kinds of live
/// update are new variants here**, so every consumer reads one shape.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// A job of theirs changed state.
    Job(JobView),
    /// A running render or preview of theirs got further (#698): the same
    /// shape as its [`JobView::progress`], at most twice a second and only
    /// when the percentage or the phase moved. Its end is a [`Event::Job`].
    JobProgress {
        /// The job.
        id: i64,
        /// How far it has got.
        progress: ProgressView,
    },
    /// One of their projects was changed — by the assistant, so far: re-read
    /// it, and the preview refreshes as the edits land.
    Project {
        /// The project.
        id: i64,
        /// Its revision now.
        revision: i64,
    },
    /// An assistant turn started, was charged for a call, or ended — with
    /// the balance left after it.
    ChatTurn {
        /// The turn as it stands.
        turn: TurnView,
        /// Their balance, in micro-dollars.
        balance_micros: i64,
    },
    /// More of the words the assistant is writing in `turn`, in order: append
    /// them to what it said last.
    ChatText {
        /// The turn.
        turn: i64,
        /// The words.
        text: String,
    },
    /// A tool the assistant is calling, and then how it answered.
    ChatTool {
        /// The turn.
        turn: i64,
        /// The tool.
        tool: String,
        /// `running`, then `answered` or `refused`.
        state: &'static str,
        /// What it said, shortened, once it has answered.
        said: Option<String>,
    },
    /// A paid tool's quote, waiting for the user's yes or no
    /// (`POST /api/chat/turns/{turn}/quote`). The model cannot answer it.
    ChatQuote {
        /// The turn it was quoted in.
        turn: i64,
        /// What it would cost, in words and figures.
        quote: QuoteView,
    },
    /// Events were dropped before this reader saw them: re-read what is shown.
    Resync,
}

/// The server's event bus: [`send`](Events::send) to one user,
/// [`subscribe`](Events::subscribe) as one user.
///
/// Cheap to clone; every clone is the same bus.
#[derive(Clone)]
pub struct Events {
    sender: broadcast::Sender<(UserId, Arc<Event>)>,
    closing: Arc<watch::Sender<bool>>,
}

impl Default for Events {
    fn default() -> Self {
        Self::new()
    }
}

impl Events {
    /// A bus nobody is listening to yet.
    pub fn new() -> Self {
        Self {
            sender: broadcast::channel(BACKLOG).0,
            closing: Arc::new(watch::channel(false).0),
        }
    }

    /// Tell `user` about `event`. Nobody listening is not an error: the
    /// database already holds whatever changed.
    pub fn send(&self, user: UserId, event: Event) {
        let _ = self.sender.send((user, Arc::new(event)));
    }

    /// End every subscription, so a graceful shutdown is not held open by
    /// streams that never finish on their own.
    pub fn close(&self) {
        self.closing.send_replace(true);
    }

    /// Everything sent to `user` from now until the bus closes.
    pub fn subscribe(&self, user: UserId) -> impl Stream<Item = Event> + Send + use<> {
        let state = (self.sender.subscribe(), self.closing.subscribe());
        futures_util::stream::unfold(state, move |(mut events, mut closing)| async move {
            loop {
                let received = tokio::select! {
                    _ = closing.wait_for(|closed| *closed) => return None,
                    received = events.recv() => received,
                };
                let event = match received {
                    Ok((to, event)) if to == user => (*event).clone(),
                    Ok(_) => continue,
                    Err(broadcast::error::RecvError::Lagged(_)) => Event::Resync,
                    Err(broadcast::error::RecvError::Closed) => return None,
                };
                return Some((event, (events, closing)));
            }
        })
    }
}
