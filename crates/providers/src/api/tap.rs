//! A copy of every reply a caller reads, for the one caller that keeps them.
//!
//! The live provider check ([`crate::live`], #567) has two jobs: say whether
//! each vendor still answers in the shape our clients parse, and hand back the
//! real bodies it received so a hand-written fixture can be replaced by a
//! captured one. The second job needs the bytes exactly as they arrived —
//! before serde has thrown away every field we do not name — and it needs them
//! from **the same client code the product runs**, because a check that built
//! its own requests would test itself rather than the clients.
//!
//! So the copy is taken at the transport: a [`Caller`](super::http::Caller)
//! given a [`Tap`] appends every body it reads — a success, a refusal, a
//! stream as it streams — and behaves otherwise exactly as it did. Nothing in
//! the product sets one.

use std::io::Read;
use std::sync::{Arc, Mutex, PoisonError};

/// One reply, as it arrived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exchange {
    /// What was called.
    pub url: String,
    /// The status it answered with.
    pub status: u16,
    /// The body, byte for byte — or, once [`crate::live`] has scrubbed it,
    /// with every credential in it replaced.
    pub body: Vec<u8>,
}

/// Where a tapped caller copies what it reads. Cloning shares the record.
#[derive(Debug, Clone, Default)]
pub struct Tap(Arc<Mutex<Vec<Exchange>>>);

impl Tap {
    /// An empty record.
    pub fn new() -> Self {
        Self::default()
    }

    /// Everything recorded so far, leaving the record empty.
    pub fn take(&self) -> Vec<Exchange> {
        std::mem::take(&mut *self.lock())
    }

    /// Records a reply; returns its place, for a stream to add to.
    pub(crate) fn record(&self, url: &str, status: u16, body: Vec<u8>) -> usize {
        let mut exchanges = self.lock();
        exchanges.push(Exchange {
            url: url.to_owned(),
            status,
            body,
        });
        exchanges.len() - 1
    }

    /// More of the body recorded at `index`.
    fn extend(&self, index: usize, more: &[u8]) {
        if let Some(exchange) = self.lock().get_mut(index) {
            exchange.body.extend_from_slice(more);
        }
    }

    /// The record, even if a thread panicked holding it: a copy of bodies
    /// already read is still worth having.
    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Exchange>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// A reader that copies what passes through it into a [`Tap`].
pub(crate) struct Tee<R> {
    inner: R,
    tap: Tap,
    index: usize,
}

impl<R> Tee<R> {
    /// `inner`, copied into `tap` at `index` as it is read.
    pub(crate) fn new(inner: R, tap: Tap, index: usize) -> Self {
        Self { inner, tap, index }
    }
}

impl<R: Read> Read for Tee<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.inner.read(buf)?;
        self.tap.extend(self.index, &buf[..read]);
        Ok(read)
    }
}
