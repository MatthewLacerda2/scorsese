//! Which moments a sheet takes — arithmetic, so no file is opened.

use scorsese_render::contact::{Look, MAX_FRAMES, label};

/// The default a caller who says nothing gets.
#[test]
fn five_frames_five_seconds_apart_from_the_start() {
    assert_eq!(
        Look::default().moments(60.0),
        [0.0, 5.0, 10.0, 15.0, 20.0],
        "one every five seconds, five of them"
    );
}

/// The cap is enforced below the interface rather than trusted from the caller.
#[test]
fn more_than_five_is_five() {
    let asked = Look {
        count: 50,
        ..Look::default()
    };
    assert_eq!(asked.moments(600.0).len(), MAX_FRAMES);
}

#[test]
fn zero_frames_is_one_frame() {
    let asked = Look {
        count: 0,
        ..Look::default()
    };
    assert_eq!(asked.moments(600.0), [0.0]);
}

/// A file shorter than the window gives what it has, never a decode past the
/// end — and never nothing, because a two-second clip still has a frame worth
/// looking at.
#[test]
fn a_short_file_gives_the_frames_it_has() {
    assert_eq!(Look::default().moments(12.0), [0.0, 5.0, 10.0]);
    assert_eq!(Look::default().moments(2.0), [0.0]);
    assert_eq!(Look::default().moments(0.4), [0.0]);
}

/// Nobody probed it, so nobody knows how long it is. One frame beats five
/// decodes that fail past an end we cannot see.
#[test]
fn an_unknown_length_gives_the_first_moment_only() {
    assert_eq!(Look::default().moments(0.0), [0.0]);
    assert_eq!(Look::default().moments(f64::NAN), [0.0]);
}

/// Walking a long file: a following call starts where the last one stopped.
#[test]
fn a_long_file_is_walked_from_where_the_last_call_stopped() {
    let second_pass = Look {
        from_seconds: 25.0,
        ..Look::default()
    };
    assert_eq!(second_pass.moments(120.0), [25.0, 30.0, 35.0, 40.0, 45.0]);
}

/// A named span is a different question from a cadence: the caller wants to see
/// *that stretch*, so the frames spread across it, ends included.
#[test]
fn a_named_range_spreads_evenly_across_itself() {
    let asked = Look {
        from_seconds: 0.0,
        to_seconds: Some(10.0),
        count: 5,
        ..Look::default()
    };
    assert_eq!(asked.moments(60.0), [0.0, 2.5, 5.0, 7.5, 10.0]);
}

#[test]
fn a_range_running_past_the_end_stops_at_the_end() {
    let asked = Look {
        from_seconds: 8.0,
        to_seconds: Some(400.0),
        count: 3,
        ..Look::default()
    };
    let moments = asked.moments(12.0);
    assert_eq!(moments.first(), Some(&8.0));
    assert!(
        moments.last().is_some_and(|last| *last < 12.0),
        "{moments:?} must not seek past the end of a 12-second file"
    );
}

#[test]
fn a_start_past_the_end_is_pulled_back_into_the_file() {
    let asked = Look {
        from_seconds: 900.0,
        ..Look::default()
    };
    let moments = asked.moments(12.0);
    assert_eq!(moments.len(), 1);
    assert!(moments[0] < 12.0, "{moments:?} is past the end");
}

#[test]
fn a_moment_reads_as_a_timecode() {
    assert_eq!(label(0.0), "0:00");
    assert_eq!(label(7.0), "0:07");
    assert_eq!(label(65.0), "1:05");
    assert_eq!(label(3849.0), "1:04:09");
    assert_eq!(label(2.5), "0:02.5", "a tenth shows when there is one");
    assert_eq!(label(59.96), "1:00", "and rounds into the next second");
}
