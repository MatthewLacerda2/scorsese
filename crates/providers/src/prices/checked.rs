//! The day a figure was last read off the vendor's page.
//!
//! A plain civil date rather than a [`Timestamp`](scorsese_core::Timestamp),
//! because the fact being recorded is *which day somebody looked*, and an hour
//! and a minute on it would be precision nobody has. It is also
//! `const`-constructible, which is what lets the table below be a `const` at
//! all — and a table that is a `const` is one a price cannot be added to
//! without a date, because the struct has no other way to be built.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

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
        later.civil_days() - self.civil_days()
    }

    /// Whether enough time has passed that this figure is worth re-reading.
    pub fn is_stale_on(self, today: Self) -> bool {
        self.days_until(today) > STALE_AFTER_DAYS
    }

    /// Today, in UTC.
    ///
    /// From the clock and nothing else — no time crate, because the whole of
    /// what is needed is a date, and the arithmetic for that is the twenty
    /// lines below rather than a dependency tree. A clock set wrong makes this
    /// wrong, which for an informational signal is a fair trade.
    pub fn today() -> Self {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        Self::from_civil_days(
            i64::try_from(seconds / 86_400).unwrap_or(0) + DAYS_FROM_CIVIL_EPOCH_TO_UNIX,
        )
    }

    /// This date as a count of days from the algorithm's own epoch.
    ///
    /// Howard Hinnant's `days_from_civil`, which is exact for every date in
    /// the proleptic Gregorian calendar and is the standard way to do this
    /// without a calendar library. Its epoch is 0000-03-01; the constant below
    /// shifts that to 1970-01-01 where it needs to meet a clock.
    fn civil_days(self) -> i64 {
        let year = i64::from(self.year) - i64::from(self.month <= 2);
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let month = i64::from(self.month);
        let day_of_year =
            (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era
    }

    /// The inverse of [`Checked::civil_days`].
    fn from_civil_days(days: i64) -> Self {
        let era = days.div_euclid(146_097);
        let day_of_era = days - era * 146_097;
        let year_of_era =
            (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let year = year_of_era + era * 400;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let shifted_month = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
        let month = shifted_month + if shifted_month < 10 { 3 } else { -9 };
        Self {
            year: i32::try_from(year + i64::from(month <= 2)).unwrap_or(0),
            month: u32::try_from(month).unwrap_or(1),
            day: u32::try_from(day).unwrap_or(1),
        }
    }
}

/// What 1970-01-01 is, counted from the civil algorithm's 0000-03-01 epoch.
const DAYS_FROM_CIVIL_EPOCH_TO_UNIX: i64 = 719_468;

impl fmt::Display for Checked {
    /// `YYYY-MM-DD`, so a column of these sorts the way it reads.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}
