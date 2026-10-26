//! The server's log, kept for a test to read (#1520).

use std::io::Write;
use std::sync::{Arc, Mutex};

use tracing::subscriber::DefaultGuard;

/// Everything logged on this thread while its guard lives.
#[derive(Clone, Default)]
pub(crate) struct Kept(Arc<Mutex<Vec<u8>>>);

impl Kept {
    /// What was logged so far.
    pub(crate) fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().expect("no test panics holding the log"))
            .into_owned()
    }
}

impl Write for Kept {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .expect("no test panics holding the log")
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Log as the server does, at its default filter, into what is returned,
/// until the guard drops. The test's runtime is one thread, so the server
/// and the worker it spawns log here too.
pub(crate) fn keep() -> (DefaultGuard, Kept) {
    let kept = Kept::default();
    let writer = kept.clone();
    let subscriber =
        scorsese_server::logs::subscriber(scorsese_server::logs::filter(None), move || {
            writer.clone()
        });
    (tracing::subscriber::set_default(subscriber), kept)
}
