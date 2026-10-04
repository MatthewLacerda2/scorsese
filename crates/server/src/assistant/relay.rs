//! Passing a reply to the browser as it streams, without flooding the bus.
//!
//! The model streams words a few characters at a time, and the event bus is
//! one channel every user shares, with a bounded backlog (`crate::events`). So
//! words are gathered and sent every [`EVERY`] or at the end of a block, and a
//! progress note — a sentence or two — is sent whole, once written.

use std::time::{Duration, Instant};

use scorsese_providers::chat::Streamed;

use crate::db::UserId;
use crate::events::{Event, Events};

/// How often gathered words are sent while a block is being written.
const EVERY: Duration = Duration::from_millis(150);

/// One turn's words on their way to its owner's browser.
pub(super) struct Relay {
    events: Events,
    user: UserId,
    turn: i64,
    text: String,
    note: String,
    sent_at: Instant,
    /// Whether anything reached the browser — after which a failed call is
    /// not retried, since the browser would hear its words twice.
    pub(super) heard: bool,
}

impl Relay {
    /// A relay for `turn` of `user`'s.
    pub(super) fn new(events: Events, user: UserId, turn: i64) -> Self {
        Self {
            events,
            user,
            turn,
            text: String::new(),
            note: String::new(),
            sent_at: Instant::now(),
            heard: false,
        }
    }

    /// Take in one piece of the reply.
    pub(super) fn hear(&mut self, piece: Streamed<'_>) {
        match piece {
            Streamed::Text(text) => {
                self.text.push_str(text);
                if self.sent_at.elapsed() >= EVERY {
                    self.flush();
                }
            }
            Streamed::Progress(text) => self.note.push_str(text),
            Streamed::BlockEnd => self.flush(),
        }
    }

    /// Send whatever is gathered.
    pub(super) fn flush(&mut self) {
        if !self.text.is_empty() {
            let text = std::mem::take(&mut self.text);
            self.send(Event::ChatText {
                turn: self.turn,
                text,
            });
        }
        let note = std::mem::take(&mut self.note);
        if !note.trim().is_empty() {
            self.send(Event::ChatProgress {
                turn: self.turn,
                text: note.trim().to_owned(),
            });
        }
        self.sent_at = Instant::now();
    }

    fn send(&mut self, event: Event) {
        self.heard = true;
        self.events.send(self.user, event);
    }
}
