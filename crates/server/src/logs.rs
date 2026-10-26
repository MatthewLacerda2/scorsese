//! The server's log (#1520): `tracing` events, written as plain lines on
//! stderr for a person or a Claude reading `docker compose logs server`.
//!
//! Every request runs in a `request` span naming its method, its matched
//! route (never the raw path, which carries ids) and, once signed in, the
//! account by id alone; it ends with one line saying its status and how long
//! it took, at `error` for a 5xx. The health check, asked every 15 seconds,
//! is routed outside it and writes nothing. Every job runs in a `job` span
//! and every assistant turn in a `turn` span, so whatever goes wrong inside
//! one names it. The spans are at `error`, so every filter but `off` keeps
//! them and a warning still says whose it is.
//!
//! [`LOG_FILTER`] picks what is written, as `info,scorsese_server=debug`
//! does: `info` when unset or blank.

use std::time::Duration;

use axum::extract::MatchedPath;
use axum::http::{Request, Response};
use tracing::field::Empty;
use tracing::{Level, Span, Subscriber};
use tracing_subscriber::filter::Targets;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::Registry;

use crate::config::LOG_FILTER;
use crate::db::UserId;

/// The route the container's health check asks, under `/api`: the router
/// leaves it out of the request log.
pub const UNLOGGED: &str = "/health";

/// Log to stderr for the rest of the process, filtered by [`LOG_FILTER`].
/// A filter that does not parse is said, and `info` is used instead.
#[expect(
    clippy::disallowed_methods,
    reason = "a log filter is no key, and it is read before the configuration so a refused one is logged"
)]
pub fn init() {
    let text = std::env::var(LOG_FILTER).ok();
    let installed = tracing::subscriber::set_global_default(subscriber(
        filter(text.as_deref()),
        std::io::stderr,
    ));
    let refused = given(text.as_deref()).is_some_and(|text| text.parse::<Targets>().is_err());
    if installed.is_ok() && refused {
        tracing::warn!(
            "{LOG_FILTER} is not a filter like `info,scorsese_server=debug`; logging at info"
        );
    }
}

/// What `text`, a [`LOG_FILTER`] value, lets through: `info` when it is
/// absent, blank or does not parse.
pub fn filter(text: Option<&str>) -> Targets {
    given(text)
        .and_then(|text| text.parse().ok())
        .unwrap_or_else(|| Targets::new().with_default(Level::INFO))
}

/// `text` unless it is blank, as compose passes a variable `.env` leaves
/// empty: a blank filter would let nothing through.
fn given(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|text| !text.is_empty())
}

/// The server's log, writing each line through `writer`.
pub fn subscriber<W>(filter: Targets, writer: W) -> impl Subscriber + Send + Sync
where
    W: for<'w> MakeWriter<'w> + Send + Sync + 'static,
{
    // The filter alone decides: `fmt()`'s builder would keep its own `info`
    // ceiling under it, and no `debug` line would ever be written.
    Registry::default()
        .with(tracing_subscriber::fmt::layer().with_writer(writer))
        .with(filter)
}

/// The span a request runs in.
pub fn request<B>(request: &Request<B>) -> Span {
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or("(no route)", MatchedPath::as_str);
    tracing::error_span!("request", method = %request.method(), route = %route, user = Empty)
}

/// Name the account the current request is from, by id alone.
pub fn signed_in(user: UserId) {
    Span::current().record("user", user.get());
}

/// The line a request ends with: its status and how long it took.
pub fn answered<B>(response: &Response<B>, latency: Duration, _span: &Span) {
    let status = response.status().as_u16();
    let ms = u64::try_from(latency.as_millis()).unwrap_or(u64::MAX);
    if response.status().is_server_error() {
        tracing::error!(status, ms, "answered");
    } else {
        tracing::info!(status, ms, "answered");
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::{Arc, Mutex};

    use tracing::{Level, Subscriber};
    use tracing_subscriber::filter::Targets;

    use super::{filter, subscriber};

    /// The assembled log under `text`, and what it has written so far.
    pub(crate) fn kept(text: &str) -> (impl Subscriber + Send + Sync, Arc<Mutex<Vec<u8>>>) {
        let kept = Arc::new(Mutex::new(Vec::new()));
        let writer = Arc::clone(&kept);
        let assembled = subscriber(filter(Some(text)), move || Kept(Arc::clone(&writer)));
        (assembled, kept)
    }

    /// What `kept` holds, as text.
    pub(crate) fn text(kept: &Mutex<Vec<u8>>) -> String {
        String::from_utf8(kept.lock().unwrap().clone()).unwrap()
    }

    /// What the assembled log writes while `log` runs, under `text`.
    fn written(filter: &str, log: impl FnOnce()) -> String {
        let (assembled, kept) = kept(filter);
        tracing::subscriber::with_default(assembled, log);
        text(&kept)
    }

    struct Kept(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for Kept {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_debug_filter_writes_debug_lines() {
        let log = written("info,scorsese_server=debug", || {
            tracing::debug!("a debug line");
            tracing::trace!("a trace line");
        });
        assert!(log.contains("a debug line"), "{log}");
        assert!(!log.contains("a trace line"), "{log}");
    }

    #[test]
    fn a_warn_filter_keeps_the_span_a_warning_is_in() {
        let log = written("warn", || {
            let span = tracing::error_span!("job", id = 7);
            span.in_scope(|| {
                tracing::info!("an info line");
                tracing::warn!("a warning");
            });
        });
        assert!(log.contains("job{id=7}"), "{log}");
        assert!(log.contains("a warning"), "{log}");
        assert!(!log.contains("an info line"), "{log}");
    }

    fn at(targets: &Targets, target: &str) -> Option<Level> {
        [
            Level::TRACE,
            Level::DEBUG,
            Level::INFO,
            Level::WARN,
            Level::ERROR,
        ]
        .into_iter()
        .find(|level| targets.would_enable(target, level))
    }

    #[test]
    fn unset_blank_or_garbled_is_info() {
        for text in [None, Some(""), Some("  "), Some("scorsese_server=loud")] {
            assert_eq!(
                at(&filter(text), "scorsese_server"),
                Some(Level::INFO),
                "{text:?}"
            );
        }
    }

    #[test]
    fn a_filter_names_a_level_per_target() {
        let targets = filter(Some("warn,scorsese_server=debug"));
        assert_eq!(at(&targets, "scorsese_server::jobs"), Some(Level::DEBUG));
        assert_eq!(at(&targets, "sqlx::query"), Some(Level::WARN));
    }
}
