//! The day a figure was last read off the vendor's page.
//!
//! A plain civil date rather than a [`Timestamp`](scorsese_core::Timestamp),
//! because the fact being recorded is *which day somebody looked*, and an hour
//! and a minute on it would be precision nobody has. It is also
//! `const`-constructible, which is what lets the table below be a `const` at
//! all — and a table that is a `const` is one a price cannot be added to
//! without a date, because the struct has no other way to be built.

use std::fmt;

use scorsese_core::civil;

/// How long a figure may go unchecked before it is worth mentioning.
///
/// Six months, and the number is a judgement rather than a rule: vendors
/// reprice on no schedule at all, so any threshold is arbitrary. What is not
/// arbitrary is that this only ever *informs* — CLAUDE.md's gates-vs-signals
/// rule, and a price being old genuinely does not make it wrong.
pub const STALE_AFTER_DAYS: i64 = 180;

/// A day, in UTC, with no time on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Checked {
    year: i32,
    month: u32,
    day: u32,
}

impl Checked {
    /// The day somebody read this figure off the vendor's page.
    ///
    /// Not validated, and deliberately: this is only ever called with a
    /// literal a person typed while looking at a price, so the failure it
    /// could catch — February the thirty-first — is one nobody has ever made
    /// here. What it would cost is the `const`, and with it the guarantee that
    /// a rate cannot exist without a date.
    pub const fn on(year: i32, month: u32, day: u32) -> Self {
        Self { year, month, day }
    }

    /// Days from this date to `later`, negative if `later` is earlier.
    pub fn days_until(self, later: Self) -> i64 {
        later.day() - self.day()
    }

    /// Whether enough time has passed that this figure is worth re-reading.
    pub fn is_stale_on(self, today: Self) -> bool {
        self.days_until(today) > STALE_AFTER_DAYS
    }

    /// Today, in UTC.
    ///
    /// From the clock and nothing else — no time crate, because the whole of
    /// what is needed is a date, and core's [`civil`] arithmetic is that. A
    /// clock set wrong makes this wrong, which for an informational signal is
    /// a fair trade.
    pub fn today() -> Self {
        let (year, month, day) = civil::date_of_day(civil::today().unwrap_or(0));
        Self { year, month, day }
    }

    /// This date as days since 1970-01-01.
    fn day(self) -> i64 {
        civil::day_of_date(self.year, self.month, self.day)
    }
}

impl fmt::Display for Checked {
    /// `YYYY-MM-DD`, so a column of these sorts the way it reads.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}
