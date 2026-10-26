//! Turning a count of seconds or days since the Unix epoch into a calendar
//! date, and a calendar date back into days.
//!
//! Public as `scorsese_core::civil` (#1279) because a plain date is wanted
//! outside this crate too — the day a price was checked, the day a bank
//! bulletin is asked for — and this is the one copy of the arithmetic.
//!
//! Twenty lines rather than a dependency, and it earns them: the whole of what
//! this crate needs from a clock is "what UTC instant is it", and every calendar
//! crate that answers that also brings time zones, parsing, formatting and a
//! dependency tree — none of which anything here would use.
//!
//! The algorithm is Howard Hinnant's `civil_from_days`, exact for every date in
//! the proleptic Gregorian calendar. **Leap years are the reason to use a known
//! one rather than write one**: 2000 is a leap year and 1900 is not, and every
//! naive every-fourth-year count gets one of those wrong.

/// What 1970-01-01 is, counted from the algorithm's own 0000-03-01 epoch.
const DAYS_TO_UNIX_EPOCH: i64 = 719_468;

/// Seconds in a day. No leap seconds: UTC has them and Unix time does not,
/// which is the vendor's convention as well as the format's.
pub const SECONDS_A_DAY: i64 = 86_400;

/// Today, as days since 1970-01-01 in UTC, from
/// [`Timestamp::unix_now`](super::Timestamp::unix_now); `None` on a clock set
/// before 1970.
pub fn today() -> Option<i64> {
    Some(super::Timestamp::unix_now()?.div_euclid(SECONDS_A_DAY))
}

/// The civil date, `(year, month, day)`, `days` after 1970-01-01.
pub fn date_of_day(days: i64) -> (i32, u32, u32) {
    civil_from_days(days + DAYS_TO_UNIX_EPOCH)
}

/// How many days `(year, month, day)` is after 1970-01-01, negative before it.
///
/// Hinnant's `days_from_civil`, the inverse of [`date_of_day`]. Not validated:
/// February the thirty-first counts as March the third.
pub fn day_of_date(year: i32, month: u32, day: u32) -> i64 {
    let year = i64::from(year) - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - DAYS_TO_UNIX_EPOCH
}

/// A UTC instant, split the way [`Timestamp::from_utc`](super::Timestamp::from_utc)
/// takes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Utc {
    pub(super) year: i32,
    pub(super) month: u32,
    pub(super) day: u32,
    pub(super) hour: u32,
    pub(super) minute: u32,
    pub(super) second: u32,
}

impl Utc {
    /// The instant `seconds` after the Unix epoch.
    pub(super) fn from_unix(seconds: i64) -> Self {
        let days = seconds.div_euclid(SECONDS_A_DAY);
        let within = seconds.rem_euclid(SECONDS_A_DAY);
        let (year, month, day) = date_of_day(days);
        Self {
            year,
            month,
            day,
            hour: (within / 3600) as u32,
            minute: ((within / 60) % 60) as u32,
            second: (within % 60) as u32,
        }
    }
}

/// The civil date `days` after 0000-03-01.
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    // March-based, which is what makes the leap day the last of the year and so
    // the reason this arithmetic has no special case in it at all.
    let shifted = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted + 2) / 5 + 1;
    let month = shifted + if shifted < 10 { 3 } else { -9 };
    (
        i32::try_from(year + i64::from(month <= 2)).unwrap_or(0),
        u32::try_from(month).unwrap_or(1),
        u32::try_from(day).unwrap_or(1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_and_dates_turn_into_each_other_across_the_leap_rules() {
        for (days, date) in [
            (0, (1970, 1, 1)),
            (-1, (1969, 12, 31)),
            (11_016, (2000, 2, 29)),
            (-25_508, (1900, 3, 1)),
            (20_740, (2026, 10, 14)),
            (19_753, (2024, 1, 31)),
            (19_722, (2023, 12, 31)),
            (10_651, (1999, 3, 1)),
            (47_540, (2100, 2, 28)),
            (47_541, (2100, 3, 1)),
            (157_113, (2400, 2, 29)),
            (-135_140, (1600, 1, 1)),
            (-719_162, (1, 1, 1)),
        ] {
            assert_eq!(date_of_day(days), date, "{days}");
            assert_eq!(day_of_date(date.0, date.1, date.2), days, "{date:?}");
        }
        assert_eq!(day_of_date(1900, 3, 1) - day_of_date(1900, 2, 28), 1);
    }

    #[test]
    fn an_instant_splits_into_its_date_and_time_of_day() {
        let at = |seconds| {
            let utc = Utc::from_unix(seconds);
            (
                utc.year, utc.month, utc.day, utc.hour, utc.minute, utc.second,
            )
        };
        assert_eq!(at(1_234_567_890), (2009, 2, 13, 23, 31, 30));
        assert_eq!(at(86_399), (1970, 1, 1, 23, 59, 59));
        assert_eq!(at(-3_661), (1969, 12, 31, 22, 58, 59));
    }

    #[test]
    fn today_is_the_clock_s_day() {
        let day = || super::super::Timestamp::unix_now().map(|s| s.div_euclid(SECONDS_A_DAY));
        let (before, read, after) = (day(), today(), day());
        assert!(
            before <= read && read <= after,
            "{before:?} {read:?} {after:?}"
        );
        assert!(read > Some(20_000), "after 2024: {read:?}");
    }
}
