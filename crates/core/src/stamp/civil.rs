//! Turning a count of seconds since the Unix epoch into a calendar date.
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
const SECONDS_A_DAY: i64 = 86_400;

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
        let (year, month, day) = civil_from_days(days + DAYS_TO_UNIX_EPOCH);
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
