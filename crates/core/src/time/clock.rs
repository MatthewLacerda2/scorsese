//! Seconds as clock text: the one way every surface writes a time for a person.

/// How finely [`clock`] reads a time, which is the caller's to choose because
/// the right answer depends on what the time *is*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    /// Whole seconds, rounded down — for a **position** (a playhead, a ruler
    /// tick). At 59.9s the playhead is not at a minute yet.
    Floor,
    /// Whole seconds, rounded to the nearest — for a **length**. A 59.6s clip
    /// runs a minute to anybody reading its badge.
    Round,
    /// Tenths, rounded to the nearest — where the reading sits beside an exact
    /// frame count, or names a moment finer than a second.
    Tenths,
}

/// Seconds as a clock reads them: `0:59`, `2:30`, `1:01:40`, or `0:04.5` in
/// [`Precision::Tenths`].
///
/// `m:ss` under an hour and `h:mm:ss` from one; the first field is never
/// zero-padded, because `00:00:04` for a four-second clip is a form to be
/// filled in rather than a time. The total is rounded **before** it is split
/// into fields, so 59.6s rounded is `1:00` and never `0:60`. Anything below
/// zero, and NaN, reads as zero.
///
/// This is timeline time as shown to a person. It is unrelated to
/// [`Timestamp`](crate::Timestamp), which is a civil instant (`when did this
/// happen`) and not a duration or a position on the edit.
pub fn clock(seconds: f64, precision: Precision) -> String {
    let seconds = seconds.max(0.0);
    let (whole, tenth) = match precision {
        Precision::Floor => (seconds.floor() as u64, None),
        Precision::Round => (seconds.round() as u64, None),
        Precision::Tenths => {
            let tenths = (seconds * 10.0).round() as u64;
            (tenths / 10, Some(tenths % 10))
        }
    };
    let (hours, minutes, secs) = (whole / 3600, (whole / 60) % 60, whole % 60);
    let mut text = if hours > 0 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes}:{secs:02}")
    };
    if let Some(tenth) = tenth {
        text.push_str(&format!(".{tenth}"));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bug this exists for: splitting before rounding printed `0:60`.
    #[test]
    fn a_length_just_under_a_minute_runs_a_minute_and_a_position_does_not() {
        assert_eq!(clock(59.6, Precision::Round), "1:00");
        assert_eq!(clock(59.6, Precision::Floor), "0:59");
    }

    #[test]
    fn hours_appear_only_once_there_are_hours() {
        assert_eq!(clock(3700.0, Precision::Round), "1:01:40");
        assert_eq!(clock(3599.0, Precision::Floor), "59:59");
        assert_eq!(clock(3600.0, Precision::Floor), "1:00:00");
    }

    #[test]
    fn tenths_are_rounded_into_the_next_second() {
        assert_eq!(clock(59.96, Precision::Tenths), "1:00.0");
        assert_eq!(clock(4.5, Precision::Tenths), "0:04.5");
        assert_eq!(clock(3723.4, Precision::Tenths), "1:02:03.4");
    }

    #[test]
    fn nothing_reads_below_zero() {
        assert_eq!(clock(-3.0, Precision::Round), "0:00");
        assert_eq!(clock(f64::NAN, Precision::Tenths), "0:00.0");
    }
}
