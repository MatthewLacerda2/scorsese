//! One member's share of the stock libraries every member shares (#1652).
//!
//! Every member's `stock_search` and `stock_import`, and a pick from the
//! assistant's picker, reach Pixabay on the operator's one key, and
//! LottieFiles from the server's one address. Pixabay allows a key 100
//! requests a minute ("By default, you can make up to 100 requests per 60
//! seconds", pixabay.com/api/docs, read 2026-11-02), so one member left alone
//! could keep it refused for everyone. Each library a member reaches is
//! wrapped in [`Shared`], which counts that member's searches and lookups by
//! id against [`REQUESTS_PER_MINUTE`] before each goes out, the same share for
//! every library, and refuses the rest in words that say when to ask again.
//!
//! Downloads are not counted: they come from the libraries' file hosts, which
//! the key's limit does not count, and every one follows a lookup that was.
//! A result found in the member's stock cache never reaches the library, so
//! it costs nothing here either.

use std::sync::Arc;
use std::time::Duration;

use scorsese_mcp::{Libraries, Stock};
use scorsese_providers::stock::{self, Candidate, Library, Medium, Page, Query, StockError};

use super::Toolbox;
use crate::db::UserId;
use crate::http::limits::Limits;

/// How many requests one member may send each stock library a minute: under
/// a third of Pixabay's 100, so three members at their busiest still leave
/// room for everyone else.
pub(super) const REQUESTS_PER_MINUTE: u32 = 30;

/// How many requests each member has sent each library this minute.
#[derive(Debug, Clone)]
pub(crate) struct Shares {
    most: u32,
    sent: Limits<(UserId, &'static str)>,
}

impl Default for Shares {
    fn default() -> Self {
        Self::new(REQUESTS_PER_MINUTE)
    }
}

impl Shares {
    /// At most `most` requests a minute for each member, to each library.
    pub(crate) fn new(most: u32) -> Self {
        Self {
            most,
            sent: Limits::new(most),
        }
    }

    /// Count one request of `user`'s to `library`, or say why it may not go.
    fn take(&self, user: UserId, library: &'static str) -> Result<(), StockError> {
        self.sent
            .take((user, library), 1)
            .map_err(|wait| StockError::Share {
                library,
                most: self.most,
                seconds: seconds(wait),
            })
    }
}

/// `wait` in whole seconds, rounded up, and never none: "try again in 0
/// seconds" would be refused again.
fn seconds(wait: Duration) -> u64 {
    let whole = wait.as_secs() + u64::from(wait.subsec_nanos() > 0);
    whole.max(1)
}

/// A library one member reaches, counted against their share of it.
struct Shared {
    inner: Stock,
    user: UserId,
    shares: Shares,
}

impl Library for Shared {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn page(&self, query: &Query, page: u32) -> Result<Page, StockError> {
        self.shares.take(self.user, self.inner.name())?;
        self.inner.page(query, page)
    }

    fn one(&self, medium: Medium, id: u64) -> Result<Option<Candidate>, StockError> {
        self.shares.take(self.user, self.inner.name())?;
        self.inner.one(medium, id)
    }

    fn hosts(&self) -> &'static [&'static str] {
        self.inner.hosts()
    }

    fn download(
        &self,
        url: &str,
        limit: u64,
        to: &mut dyn std::io::Write,
    ) -> Result<u64, StockError> {
        self.inner.download(url, limit, to)
    }
}

impl Shares {
    /// `library`, as `user` reaches it: within their share.
    pub(super) fn of(&self, user: UserId, library: Stock) -> Stock {
        Arc::new(Shared {
            inner: library,
            user,
            shares: self.clone(),
        })
    }
}

impl Toolbox {
    /// Where `user`'s stock tools find each medium's library: the toolbox's
    /// own when it was given one, and otherwise each medium's
    /// (`stock::library`), every one within `user`'s share.
    pub(super) fn libraries(&self, user: UserId) -> Libraries {
        let (given, shares) = (self.stock.clone(), self.shares.clone());
        Arc::new(move |medium| {
            let library = match &given {
                Some(library) => library.clone(),
                None => stock::library(medium)
                    .map(Arc::from)
                    .map_err(|error| format!("{error}"))?,
            };
            Ok(shares.of(user, library))
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::seconds;

    #[test]
    fn a_wait_is_said_in_whole_seconds_rounded_up() {
        assert_eq!(seconds(Duration::from_millis(41_200)), 42);
        assert_eq!(seconds(Duration::from_secs(12)), 12);
        assert_eq!(seconds(Duration::ZERO), 1);
    }
}
