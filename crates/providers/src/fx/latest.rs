//! The latest rate, kept rather than fetched per purchase (the maintainer,
//! 2026-10-14, on #548).
//!
//! Whoever holds a [`Latest`] calls [`Latest::refresh`] every
//! [`REFRESH_EVERY`]; a purchase reads [`Latest::current`], which answers
//! only while the rate is at most [`USABLE_FOR`] old. A failed refresh keeps
//! the last good rate: a sources' bad minute is not an outage, and ten minutes
//! of them is, which is when a purchase is refused rather than sold at a rate
//! nobody can vouch for any more.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::{FxError, Rate, Rates};

/// How often the rate is read again: two minutes.
pub const REFRESH_EVERY: Duration = Duration::from_secs(2 * 60);

/// How old a rate may be and still be sold at: ten minutes.
pub const USABLE_FOR: Duration = Duration::from_secs(10 * 60);

/// The last rate read, and when.
pub struct Latest {
    rates: Arc<dyn Rates>,
    last: Mutex<Option<(Instant, Rate)>>,
}

impl Latest {
    /// Nothing read yet from `rates`.
    pub fn new(rates: Arc<dyn Rates>) -> Self {
        Self {
            rates,
            last: Mutex::new(None),
        }
    }

    /// Read the rate again. On success it replaces the one kept; on failure
    /// the one kept stays, and the failure is returned for the log.
    /// Blocking: one or two HTTPS calls.
    pub fn refresh(&self) -> Result<Rate, FxError> {
        let rate = self.rates.usd_brl()?;
        self.keep(rate.clone(), Instant::now());
        Ok(rate)
    }

    /// The rate kept, if it was read at most [`USABLE_FOR`] ago.
    pub fn current(&self) -> Option<Rate> {
        let last = self
            .last
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        last.as_ref()
            .filter(|(at, _)| at.elapsed() <= USABLE_FOR)
            .map(|(_, rate)| rate.clone())
    }

    /// Whether any rate has been read yet.
    pub fn is_empty(&self) -> bool {
        self.last
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_none()
    }

    /// Keep `rate` as read at `at` — [`refresh`](Self::refresh)'s, and a
    /// test's way to hold an old one.
    pub fn keep(&self, rate: Rate, at: Instant) {
        *self
            .last
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some((at, rate));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;
    use crate::fx::Source;

    /// A source that answers 5.01 until told to fail.
    struct Flaky(AtomicBool);

    impl Rates for Flaky {
        fn usd_brl(&self) -> Result<Rate, FxError> {
            if self.0.load(Ordering::SeqCst) {
                return Err(FxError {
                    awesome: "429".into(),
                    ptax: "down".into(),
                });
            }
            Ok(Rate::parse("5.01", Source::Awesome).unwrap())
        }
    }

    #[test]
    fn a_failed_refresh_keeps_the_last_good_rate() {
        let source = Arc::new(Flaky(AtomicBool::new(false)));
        let latest = Latest::new(source.clone());
        assert!(latest.is_empty());
        assert_eq!(latest.current(), None);
        latest.refresh().unwrap();
        source.0.store(true, Ordering::SeqCst);
        assert!(latest.refresh().is_err());
        assert_eq!(latest.current().unwrap().brl_per_usd, "5.01");
    }

    #[test]
    fn a_rate_older_than_ten_minutes_is_not_sold_at() {
        let latest = Latest::new(Arc::new(Flaky(AtomicBool::new(true))));
        let rate = Rate::parse("5.01", Source::Ptax).unwrap();
        let read = |ago: u64| {
            Instant::now()
                .checked_sub(Duration::from_secs(ago))
                .unwrap()
        };
        latest.keep(rate.clone(), read(9 * 60 + 50));
        assert_eq!(latest.current(), Some(rate.clone()));
        latest.keep(rate, read(10 * 60 + 1));
        assert_eq!(latest.current(), None);
        assert!(!latest.is_empty());
    }
}
