//! Reading the clock into a stamp, and printing one.

use scorsese_core::Timestamp;

#[test]
fn an_instant_after_the_epoch_is_stamped_in_utc() {
    let stamp = |seconds| Timestamp::from_unix(seconds).expect("a real instant");
    assert_eq!(stamp(0).as_str(), "1970-01-01T00:00:00Z");
    assert_eq!(stamp(1_234_567_890).as_str(), "2009-02-13T23:31:30Z");
}

#[test]
fn the_clock_reads_as_now() {
    let before = Timestamp::unix_now().expect("a clock after 1970");
    let now = Timestamp::now().expect("a clock after 1970");
    let after = Timestamp::unix_now().expect("a clock after 1970");
    assert!(before > 1_700_000_000, "after 2023: {before}");
    let bounds = [before, after].map(|s| Timestamp::from_unix(s).expect("an instant"));
    assert!(bounds[0].as_str() <= now.as_str() && now.as_str() <= bounds[1].as_str());
}

#[test]
fn a_stamp_prints_as_written_and_pads_in_a_table() {
    let stamp = Timestamp::from_utc(2026, 8, 4, 14, 20, 0).expect("a real instant");
    assert_eq!(stamp.to_string(), "2026-08-04T14:20:00Z");
    assert_eq!(format!("[{stamp:>22}]"), "[  2026-08-04T14:20:00Z]");
}
