//! A provider that behaves like a long-running one and costs nothing.
//!
//! Scripted rather than clever: each asset is given a queue of answers and the
//! mock hands back the next one each time it is polled. That is what lets a
//! test say *this shot finishes on the second poll and that one is refused*
//! without any timing in it at all.
//!
//! It also **counts submissions**, which is how the money guarantee gets
//! asserted: a cache that works is a cache that produces no second submit.

use std::cell::RefCell;
use std::collections::HashMap;

use scorsese_providers::video::{Brief, Progress, ProviderError, Ready, Ticket, VideoProvider};

/// What a scripted provider does when it is next asked about a shot.
#[derive(Debug, Clone)]
pub(crate) enum Answer {
    /// Not finished yet.
    Waiting,
    /// Finished; the bytes are these.
    Ready(Vec<u8>),
    /// Finished, and refused.
    Failed(String),
}

/// A provider whose every answer was decided in advance.
#[derive(Debug, Default)]
pub(crate) struct Mock {
    script: RefCell<HashMap<String, Vec<Answer>>>,
    videos: RefCell<HashMap<String, Vec<u8>>>,
    /// Every brief handed over, in order — the record the cache is checked
    /// against.
    pub(crate) submitted: RefCell<Vec<String>>,
    /// Set to refuse the next submit outright, as an unreachable network does.
    pub(crate) unreachable: RefCell<bool>,
}

impl Mock {
    /// A provider that answers `answers` for the asset called `id`, in order.
    pub(crate) fn answering(id: &str, answers: Vec<Answer>) -> Self {
        let mock = Self::default();
        mock.script
            .borrow_mut()
            .insert(id.to_owned(), answers.into_iter().rev().collect());
        mock
    }

    /// Adds another asset's script to this one.
    pub(crate) fn and(self, id: &str, answers: Vec<Answer>) -> Self {
        self.script
            .borrow_mut()
            .insert(id.to_owned(), answers.into_iter().rev().collect());
        self
    }

    /// How many briefs have been handed over.
    pub(crate) fn submissions(&self) -> usize {
        self.submitted.borrow().len()
    }
}

/// A ticket names the asset it belongs to, so a poll can find its script
/// without the mock keeping any state of its own about which is which.
impl VideoProvider for Mock {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn submit(&self, brief: &Brief) -> Result<Ticket, ProviderError> {
        if *self.unreachable.borrow() {
            return Err(ProviderError::new("mock", "no route to host"));
        }
        self.submitted.borrow_mut().push(brief.id.to_string());
        Ok(Ticket(format!("operations/{}", brief.id)))
    }

    fn poll(&self, ticket: &Ticket) -> Result<Progress, ProviderError> {
        let id = ticket.0.trim_start_matches("operations/").to_owned();
        let next = self
            .script
            .borrow_mut()
            .get_mut(&id)
            .and_then(Vec::pop)
            .unwrap_or(Answer::Waiting);
        Ok(match next {
            Answer::Waiting => Progress::Waiting,
            Answer::Failed(message) => Progress::Failed(message),
            Answer::Ready(bytes) => {
                let uri = format!("https://mock/files/{id}");
                self.videos.borrow_mut().insert(uri.clone(), bytes);
                Progress::Ready(Ready(uri))
            }
        })
    }

    fn fetch(&self, ready: &Ready) -> Result<Vec<u8>, ProviderError> {
        self.videos
            .borrow()
            .get(&ready.0)
            .cloned()
            .ok_or_else(|| ProviderError::new("mock", format!("nothing at {}", ready.0)))
    }
}
