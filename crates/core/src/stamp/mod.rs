//! Wall-clock instants: when something happened, as opposed to when it plays.
//!
//! Nothing here is timeline time. [`crate::time`] is the grid a cut sits on and
//! is counted in frames; this is the ordinary clock, and it answers questions
//! about the *project's history* rather than about the video — when an asset
//! joined the table, when a request was handed to a provider.
//!
//! **No stamp is ever hashed into a brief, and none is ever read by the
//! compositor.** Both exclusions are load-bearing and neither is an oversight.
//! A stamp inside a brief hash would make an unchanged prompt look changed and
//! bill for it again; a stamp reaching the compositor would make a golden
//! render depend on the day it ran. The type carries no arithmetic for the same
//! reason it carries no clock: reading the current time is the business of
//! whichever crate stamps a document, not of the model that stores one.

use std::fmt;

use serde::{Deserialize, Serialize};

/// An instant, as UTC in RFC 3339: `2026-08-04T14:20:00Z`.
///
/// A string rather than a number of seconds, because `project.json` is a
/// document agents read: an epoch count is a fact nobody can check by looking,
/// and the whole format is written on the principle that what is in the file
/// explains itself.
///
/// One spelling only — UTC, whole seconds, trailing `Z`. Offsets and fractions
/// are refused rather than normalised, so that ordering these lexicographically
/// is the same as ordering them chronologically, and sorting a table by when
/// its rows arrived needs no parsing at all.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Timestamp(String);

/// Why a string is not a [`Timestamp`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "`{found}` is not an instant: write it as UTC in RFC 3339, \
     whole seconds and a trailing Z, like `2026-08-04T14:20:00Z`"
)]
pub struct TimestampError {
    /// The string that was not one.
    pub found: String,
}

mod civil;

impl Timestamp {
    /// Now, in UTC, to the second.
    ///
    /// The one place this crate reads a clock. It is `Option` rather than a
    /// panic because a machine whose clock is set before 1970 can produce a
    /// negative instant, and a bookkeeping stamp is never worth failing an edit
    /// over — an asset with no `created_at` is a document that says nothing
    /// about when it was made, which is exactly what the field being optional
    /// already means.
    ///
    /// Never hashed into a brief and never read by the compositor. A timestamp
    /// inside a brief hash would make an unchanged prompt look edited and bill
    /// for it again; one the compositor read would make a golden render depend
    /// on the day it ran.
    pub fn now() -> Option<Self> {
        Self::from_unix(Self::unix_now()?)
    }

    /// Seconds since the Unix epoch, right now.
    ///
    /// Published beside [`Timestamp::now`] because the arithmetic somebody
    /// actually wants a clock for is *how long ago* — a retention window, an
    /// age — and doing that on a formatted string means parsing it back.
    pub fn unix_now() -> Option<i64> {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|since| i64::try_from(since.as_secs()).ok())
    }

    /// The instant `seconds` after the Unix epoch, in UTC.
    pub fn from_unix(seconds: i64) -> Option<Self> {
        let at = civil::Utc::from_unix(seconds);
        Self::from_utc(at.year, at.month, at.day, at.hour, at.minute, at.second).ok()
    }

    /// The instant as `project.json` writes it.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Builds a stamp from the civil parts of a UTC instant.
    ///
    /// The way a caller that has just read a clock gets one, without having to
    /// know how the format is spelled. Out-of-range parts are refused here
    /// rather than producing a string that would not parse back.
    pub fn from_utc(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> Result<Self, TimestampError> {
        let text = format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z");
        Self::try_from(text)
    }
}

impl TryFrom<String> for Timestamp {
    type Error = TimestampError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        if well_formed(&text) {
            Ok(Self(text))
        } else {
            Err(TimestampError { found: text })
        }
    }
}

impl From<Timestamp> for String {
    fn from(stamp: Timestamp) -> Self {
        stamp.0
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(&self.0)
    }
}

/// Whether a string is the one spelling this type accepts.
///
/// Shape and range, and deliberately not more: that February has 28 days in
/// this year is a calendar question, and a document is not made coherent by
/// the model refusing to store a date somebody typed wrong. What matters here
/// is that every stored stamp sorts correctly and parses back, and shape alone
/// guarantees both.
fn well_formed(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 20 {
        return false;
    }
    let punctuation = [
        (4, b'-'),
        (7, b'-'),
        (10, b'T'),
        (13, b':'),
        (16, b':'),
        (19, b'Z'),
    ];
    if punctuation
        .iter()
        .any(|&(at, expected)| bytes[at] != expected)
    {
        return false;
    }
    let digits = [(0, 4), (5, 2), (8, 2), (11, 2), (14, 2), (17, 2)];
    if digits
        .iter()
        .any(|&(at, len)| !bytes[at..at + len].iter().all(u8::is_ascii_digit))
    {
        return false;
    }
    in_range(&text[5..7], 1, 12)
        && in_range(&text[8..10], 1, 31)
        && in_range(&text[11..13], 0, 23)
        && in_range(&text[14..16], 0, 59)
        && in_range(&text[17..19], 0, 59)
}

/// Whether a two-digit field is inside the range that field can hold.
fn in_range(field: &str, low: u32, high: u32) -> bool {
    field
        .parse::<u32>()
        .is_ok_and(|value| (low..=high).contains(&value))
}
