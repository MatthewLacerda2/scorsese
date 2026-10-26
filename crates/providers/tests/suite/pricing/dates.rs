//! The day a figure was checked, and when that becomes worth mentioning.

use scorsese_providers::prices::{Checked, STALE_AFTER_DAYS};

#[test]
fn a_date_reads_the_way_it_sorts() {
    assert_eq!(Checked::on(2026, 8, 4).to_string(), "2026-08-04");
    assert_eq!(Checked::on(999, 12, 31).to_string(), "0999-12-31");
    assert!(Checked::on(2026, 1, 9) < Checked::on(2026, 1, 10));
}

#[test]
fn days_are_counted_across_months_and_years() {
    let day = |year, month, d| Checked::on(year, month, d);
    assert_eq!(day(2026, 8, 4).days_until(day(2026, 8, 5)), 1);
    assert_eq!(day(2026, 1, 31).days_until(day(2026, 2, 1)), 1);
    assert_eq!(day(2025, 12, 31).days_until(day(2026, 1, 1)), 1);
    assert_eq!(day(2026, 8, 4).days_until(day(2027, 8, 4)), 365);
}

/// 2024 was a leap year and 1900 was not, which is the case a naive
/// every-fourth-year count gets wrong.
#[test]
fn leap_years_are_counted_the_way_the_calendar_does() {
    assert_eq!(
        Checked::on(2024, 2, 28).days_until(Checked::on(2024, 3, 1)),
        2
    );
    assert_eq!(
        Checked::on(1900, 2, 28).days_until(Checked::on(1900, 3, 1)),
        1
    );
    assert_eq!(
        Checked::on(2000, 2, 28).days_until(Checked::on(2000, 3, 1)),
        2
    );
}

#[test]
fn a_date_in_the_future_counts_backwards() {
    assert_eq!(
        Checked::on(2026, 8, 4).days_until(Checked::on(2026, 8, 1)),
        -3
    );
}

/// 180 days after 2026-01-01 is 2026-06-30, counted by hand off a calendar —
/// which is the point, since deriving the boundary from the function under
/// test would prove only that it agrees with itself.
#[test]
fn staleness_starts_the_day_after_the_mark() {
    let checked = Checked::on(2026, 1, 1);
    assert_eq!(
        checked.days_until(Checked::on(2026, 6, 30)),
        STALE_AFTER_DAYS
    );
    assert!(!checked.is_stale_on(Checked::on(2026, 6, 30)));
    assert!(checked.is_stale_on(Checked::on(2026, 7, 1)));
}

/// A clock that reads today is the whole of what the signal needs; that it
/// answers a plausible date at all is what is worth checking here.
#[test]
fn today_is_a_date_this_project_could_be_running_on() {
    let today = Checked::today();
    assert!(
        Checked::on(2025, 1, 1) < today,
        "{today} is before this repo"
    );
    assert!(today < Checked::on(2100, 1, 1), "{today} is implausible");
}
