//! Blocking work run off the async runtime, with a panic in it said rather
//! than unwound into the job.

/// Run `work` on tokio's blocking pool. A panic in it is a bug on the
/// server: it comes back as `crashed`, word for word, so each caller keeps
/// saying what it was doing ("drawing crashed", "the render crashed on the
/// server; that is a bug").
pub async fn blocking<T, F>(crashed: &str, work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    // The caller's span (its job's) goes with the work, so a line it logs
    // names the job.
    let span = tracing::Span::current();
    tokio::task::spawn_blocking(move || span.in_scope(work))
        .await
        .map_err(|_| crashed.to_owned())
}

#[cfg(test)]
mod tests {
    use tracing::Instrument;

    use super::blocking;
    use crate::logs::tests::{kept, text};

    /// The pool's threads log through the process's subscriber, so this one
    /// installs it: each test runs in a process of its own (nextest).
    #[tokio::test]
    async fn a_line_logged_in_the_work_names_the_callers_span() {
        let (assembled, kept) = kept("info");
        tracing::subscriber::set_global_default(assembled).unwrap();
        let span = tracing::error_span!("job", id = 3);
        blocking("never said", || tracing::warn!("drawn"))
            .instrument(span)
            .await
            .unwrap();
        let log = text(&kept);
        assert!(
            log.contains("job{id=3}: ") && log.contains("drawn"),
            "{log}"
        );
    }

    #[tokio::test]
    async fn a_finished_closure_answers_with_its_value() {
        assert_eq!(blocking("never said", || 7).await, Ok(7));
    }

    #[tokio::test]
    async fn a_panic_is_said_in_the_callers_words() {
        let answer: Result<(), String> =
            blocking("drawing crashed", || panic!("a bug in the closure")).await;
        assert_eq!(answer, Err("drawing crashed".to_owned()));
    }
}
