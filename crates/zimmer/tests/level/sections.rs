//! The same statistics again, a stretch at a time.

use scorsese_zimmer::level::{Cut, Profiler};
use scorsese_zimmer::{SAMPLE_RATE, bake_song, song::InlineOnly};

use super::{SLACK, common::songs, square};

/// The claim the whole table makes: a signal whose halves differ by a known
/// number of decibels says so **on the right rows**, where a whole-file mean
/// would report one unremarkable average for both.
#[test]
fn a_signal_that_changes_halfway_says_so_on_the_row_it_changed() {
    let rate = 1_000;
    let mut profiler = Profiler::sectioned(
        1,
        rate,
        vec![
            Cut {
                label: "quiet".to_owned(),
                end_seconds: 1.0,
            },
            Cut {
                label: "loud".to_owned(),
                end_seconds: 2.0,
            },
        ],
    );
    profiler.feed(&square(0.25, rate as usize));
    profiler.feed(&square(1.0, rate as usize));
    let profile = profiler.finish();

    assert_eq!(profile.sections.len(), 2);
    let means: Vec<f64> = profile
        .sections
        .iter()
        .map(|span| span.loudness.mean_dbfs.expect("audible"))
        .collect();
    assert!((means[0] + 12.04).abs() < SLACK, "a quarter is 12 dB down");
    assert!((means[1] - 0.0).abs() < SLACK, "full scale is full scale");
    // And the whole-file number is between them, which is precisely why it
    // would not have found either.
    let whole = profile.whole.loudness.mean_dbfs.expect("audible");
    assert!(whole > means[0] && whole < means[1], "whole {whole:.2}");
}

/// Rows carry the arrangement's names, because "the second chorus is the quiet
/// one" is a finding an author can act on and "seconds 24 to 32 are quiet" is a
/// fact they then have to look up.
#[test]
fn the_rows_are_named_after_the_patterns_that_made_them() {
    let mut profiler = Profiler::sectioned(
        1,
        1_000,
        vec![
            Cut {
                label: "verse".to_owned(),
                end_seconds: 0.5,
            },
            Cut {
                label: "chorus".to_owned(),
                end_seconds: 1.0,
            },
        ],
    );
    profiler.feed(&square(0.5, 1_000));
    let labels: Vec<Option<String>> = profiler
        .finish()
        .sections
        .into_iter()
        .map(|span| span.label)
        .collect();
    assert_eq!(
        labels,
        vec![Some("verse".to_owned()), Some("chorus".to_owned())]
    );
}

/// One stretch is the whole file said twice, so there is no table.
#[test]
fn a_signal_with_only_one_stretch_gets_no_rows() {
    let mut profiler = Profiler::new(1, SAMPLE_RATE);
    profiler.feed(&square(0.5, 4_000));
    assert!(profiler.finish().sections.is_empty());
}

/// A bake of a song takes its rows from the arrangement it rendered, and its
/// last row ends where the last pattern does — not where the audio does, since
/// a piece rings out past its final beat.
#[test]
fn a_bake_of_a_song_is_sectioned_by_its_arrangement() {
    let song = songs::song();
    let bake = bake_song(&song, &InlineOnly).expect("the fixture song renders");
    let sections = &bake.profile.sections;
    assert_eq!(sections.len(), 2, "two entries in the arrangement");
    assert!(
        sections
            .iter()
            .all(|span| span.label.as_deref() == Some("verse")),
        "both entries play the same pattern"
    );
    // Two beats at 120 bpm is one second, so the first row ends there.
    assert!((sections[0].to_seconds - 1.0).abs() < 0.01);
    assert!(
        bake.profile.seconds() >= 2.0,
        "and the piece rings out past it"
    );
}
