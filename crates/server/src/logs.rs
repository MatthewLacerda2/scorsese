//! The server's log (#1520): `tracing` events, written as plain lines on
//! stderr for a person or a Claude reading `docker compose logs server`.
//!
//! Every request runs in a `request` span naming its method, its matched
//! route (never the raw path, which carries ids) and, once signed in, the
//! account by id alone; it ends with one line saying its status and how long
//! it took, at `error` for a 5xx. The health check, asked every 15 seconds,
//! writes nothing. Every job runs in a `job` span and every assistant turn in
//! a `turn` span, so whatever goes wrong inside one names it.
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

use crate::config::LOG_FILTER;
use crate::db::UserId;

/// The route the container's health check asks, left out of the log.
const HEALTH: &str = "/api/health";

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
    tracing_subscriber::fmt()
        .with_writer(writer)
        .finish()
        .with(filter)
}

/// The span a request runs in; none for the health check.
pub fn request<B>(request: &Request<B>) -> Span {
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or("(no route)", MatchedPath::as_str);
    if route == HEALTH {
        return Span::none();
    }
    tracing::info_span!("request", method = %request.method(), route = %route, user = Empty)
}

/// Name the account the current request is from, by id alone.
pub fn signed_in(user: UserId) {
    Span::current().record("user", user.get());
}

/// The line a request ends with: its status and how long it took.
pub fn answered<B>(response: &Response<B>, latency: Duration, span: &Span) {
    if span.is_none() {
        return;
    }
    let status = response.status().as_u16();
    let ms = u64::try_from(latency.as_millis()).unwrap_or(u64::MAX);
    if response.status().is_server_error() {
        tracing::error!(status, ms, "answered");
    } else {
        tracing::info!(status, ms, "answered");
    }
}

#[cfg(test)]
mod tests {
    use tracing::Level;
    use tracing_subscriber::filter::Targets;

    use super::filter;

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
