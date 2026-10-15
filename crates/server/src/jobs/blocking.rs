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
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|_| crashed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::blocking;

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
