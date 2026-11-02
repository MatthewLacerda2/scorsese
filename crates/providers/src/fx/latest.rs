//! The latest rate, kept rather than fetched per purchase (the maintainer,
//! 2026-10-14, on #548).
//!
//! Whoever holds a [`Latest`] calls [`Latest::refresh`] every
//! [`REFRESH_EVERY`]; a purchase reads [`Latest::current`], which answers
//! only while the rate is at most [`USABLE_FOR`] old. A failed refresh keeps
//! the last good rate: a sources' bad minute is not an outage, and ten minutes
//! of them is, which is when a purchase is refused rather than sold at a rate
//! nobody can vouch for any more.
//!
//! A rate read is kept only when a second figure agrees with it
//! ([`Rate::agrees_with`]): the PTAX when the rate is AwesomeAPI's and the
//! PTAX can be read, the rate kept otherwise while it is still fresh enough
//! to sell at. With neither (a server's first read, or a read after
//! [`USABLE_FOR`] without one, while the PTAX is down) the band
//! [`Rate::parse`] holds it to is the only check, so an old rate never locks
//! out a new one. A rate that disagrees is not kept, so one source
//! answering wrong cannot move the price; past [`USABLE_FOR`] of
//! disagreement a purchase is refused, as it is when no source answers.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::{AGREEMENT_PERCENT, FxError, Rate, Rates, Source};

/// How often the rate is read again: two minutes.
pub const REFRESH_EVERY: Duration = Duration::from_secs(2 * 60);

/// How old a rate may be and still be sold at: ten minutes.
pub const USABLE_FOR: Duration = Duration::from_secs(10 * 60);

/// Why no rate was kept.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RefreshError {
    /// No source gave a rate.
    #[error(transparent)]
    Unread(#[from] FxError),
    /// A source gave one the second figure disagrees with.
    #[error(
        "{} said {} reais a dollar and {} said {}, more than {AGREEMENT_PERCENT}% apart; \
         the last rate kept stays",
        rate.source.name(),
        rate.brl_per_usd,
        check.source.name(),
        check.brl_per_usd
    )]
    Disagrees {
        /// The rate read.
        rate: Rate,
        /// What it was checked against.
        check: Rate,
    },
}

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

    /// Read the rate again and check it. When it agrees with the PTAX, or
    /// with the rate kept while that is still [`current`](Self::current),
    /// it replaces the one kept; otherwise the one
    /// kept stays, and why is returned for the log. Blocking: one or two
    /// HTTPS calls.
    pub fn refresh(&self) -> Result<Rate, RefreshError> {
        let rate = self.rates.usd_brl()?;
        let ptax = match rate.source {
            Source::Awesome => self.rates.ptax(),
            Source::Ptax => None,
        };
        if let Some(check) = ptax.or_else(|| self.current())
            && !rate.agrees_with(&check)
        {
            return Err(RefreshError::Disagrees { rate, check });
        }
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

    /// AwesomeAPI answering one figure, the PTAX another (`None`: down).
    struct Both(&'static str, Option<&'static str>);

    impl Rates for Both {
        fn usd_brl(&self) -> Result<Rate, FxError> {
            Ok(Rate::parse(self.0, Source::Awesome).unwrap())
        }

        fn ptax(&self) -> Option<Rate> {
            self.1.map(|text| Rate::parse(text, Source::Ptax).unwrap())
        }
    }

    #[test]
    fn an_awesome_rate_is_kept_only_when_the_ptax_agrees() {
        let agreeing = Latest::new(Arc::new(Both("5.30", Some("5.20"))));
        assert_eq!(agreeing.refresh().unwrap().brl_per_usd, "5.30");

        // Nothing kept yet: the PTAX alone says no.
        let far = Latest::new(Arc::new(Both("5.30", Some("5.00"))));
        let error = far.refresh().unwrap_err();
        assert!(matches!(error, RefreshError::Disagrees { .. }), "{error}");
        assert!(error.to_string().contains("bcb_ptax said 5.00"), "{error}");
        assert!(far.is_empty());

        // The PTAX is checked before the rate kept last.
        far.keep(
            Rate::parse("5.30", Source::Awesome).unwrap(),
            Instant::now(),
        );
        assert!(far.refresh().is_err());

        // With the PTAX down and nothing kept, the band is the only check.
        let alone = Latest::new(Arc::new(Both("5.30", None)));
        assert_eq!(alone.refresh().unwrap().brl_per_usd, "5.30");
    }

    /// The PTAX standing in for AwesomeAPI, and answering its own check.
    struct PtaxOnly(&'static str);

    impl Rates for PtaxOnly {
        fn usd_brl(&self) -> Result<Rate, FxError> {
            Ok(Rate::parse(self.0, Source::Ptax).unwrap())
        }

        fn ptax(&self) -> Option<Rate> {
            Some(Rate::parse(self.0, Source::Ptax).unwrap())
        }
    }

    #[test]
    fn the_ptax_standing_in_is_checked_against_the_rate_kept_not_itself() {
        let latest = Latest::new(Arc::new(PtaxOnly("5.30")));
        latest.keep(
            Rate::parse("6.00", Source::Awesome).unwrap(),
            Instant::now(),
        );
        let error = latest.refresh().unwrap_err();
        assert!(matches!(error, RefreshError::Disagrees { .. }), "{error}");
        assert_eq!(latest.current().unwrap().brl_per_usd, "6.00");
    }

    #[test]
    fn a_rate_kept_too_long_ago_to_sell_at_checks_nothing() {
        let latest = Latest::new(Arc::new(Both("5.30", None)));
        let eleven_minutes_ago = Instant::now()
            .checked_sub(Duration::from_secs(11 * 60))
            .unwrap();
        latest.keep(
            Rate::parse("2.50", Source::Awesome).unwrap(),
            eleven_minutes_ago,
        );
        assert_eq!(latest.refresh().unwrap().brl_per_usd, "5.30");
    }
}
