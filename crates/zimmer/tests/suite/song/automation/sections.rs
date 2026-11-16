//! A point placed against a named section rather than the start of the piece.
//!
//! The promise is that a build written as "eight beats before the chorus" *is*
//! the build written at the beat that sum comes to, sample for sample, and
//! that it stays before the chorus when the arrangement in front of it moves.

use scorsese_zimmer::SynthError;
use scorsese_zimmer::song::{Fit, FitMode, Param, Point, Song};

use super::setup::{at, curve, held, render};
use crate::song::marked::marker;

/// A point `beat` beats from the first downbeat of `section`.
fn against(section: &str, beat: f32, value: f32) -> Point {
    Point {
        section: Some(section.to_owned()),
        ..at(beat, value)
    }
}

/// The held fixture as a verse of two eight-beat entries and a chorus of one:
/// the chorus starts on beat 16.
fn verse_and_chorus() -> Song {
    let mut song = held(0.5);
    song.arrangement = vec![
        marker("verse"),
        "a".into(),
        "a".into(),
        marker("chorus"),
        "a".into(),
    ];
    song
}

/// `song` carrying one gain curve through `points`.
fn riding(mut song: Song, points: Vec<Point>) -> Song {
    song.automation = vec![curve(Param::Gain, points)];
    song
}

#[test]
fn a_point_against_a_section_is_the_point_at_the_beat_it_comes_to() {
    let named = riding(
        verse_and_chorus(),
        vec![against("chorus", -8.0, 0.1), against("chorus", 0.0, 0.9)],
    );
    let counted = riding(verse_and_chorus(), vec![at(8.0, 0.1), at(16.0, 0.9)]);
    assert_eq!(render(&named), render(&counted));
}

#[test]
fn a_build_into_a_section_moves_with_it_when_an_entry_goes_in_front() {
    let longer = || {
        let mut song = verse_and_chorus();
        song.arrangement.insert(1, "a".into());
        song
    };
    let named = riding(
        longer(),
        vec![against("chorus", -8.0, 0.1), against("chorus", 0.0, 0.9)],
    );
    let counted = riding(longer(), vec![at(16.0, 0.1), at(24.0, 0.9)]);
    assert_eq!(render(&named), render(&counted));
}

#[test]
fn a_point_may_run_past_the_end_of_its_section_and_mix_with_counted_ones() {
    let named = riding(
        verse_and_chorus(),
        vec![at(2.0, 0.3), against("verse", 12.0, 0.8), at(20.0, 0.2)],
    );
    let counted = riding(
        verse_and_chorus(),
        vec![at(2.0, 0.3), at(12.0, 0.8), at(20.0, 0.2)],
    );
    assert_eq!(render(&named), render(&counted));
}

/// Under `loop` the section is the one in the first pass, and the curve keeps
/// counting across the passes after it.
#[test]
fn under_a_loop_a_section_is_found_in_the_first_pass() {
    let looped = |points| {
        let mut song = riding(verse_and_chorus(), points);
        song.fit = Some(Fit::lasting(30.0, FitMode::Loop));
        song
    };
    let named = looped(vec![against("chorus", 0.0, 0.1), at(40.0, 0.9)]);
    let counted = looped(vec![at(16.0, 0.1), at(40.0, 0.9)]);
    assert_eq!(render(&named), render(&counted));
}

fn refusal(song: Song) -> String {
    song.validate()
        .expect_err("the song is refused")
        .to_string()
}

#[test]
fn a_section_the_song_does_not_have_is_refused_with_the_ones_it_does() {
    let said = refusal(riding(
        verse_and_chorus(),
        vec![against("chorus 2", -8.0, 0.1)],
    ));
    for word in ["`chorus 2`", "`verse`", "`chorus`"] {
        assert!(said.contains(word), "{word} is missing from: {said}");
    }
}

#[test]
fn a_section_in_a_song_with_no_markers_is_refused() {
    let said = refusal(riding(held(0.5), vec![against("chorus", 0.0, 0.1)]));
    assert!(said.contains("`chorus`"), "{said}");
    assert!(said.contains("no sections"), "{said}");
}

/// Order is checked once the points are placed, and the refusal names both
/// as written, since neither may say the beat it came to.
#[test]
fn points_that_do_not_ascend_once_placed_are_refused_as_written() {
    let said = refusal(riding(
        verse_and_chorus(),
        vec![at(10.0, 0.1), against("chorus", -8.0, 0.9)],
    ));
    assert!(said.contains("beat 10"), "{said}");
    assert!(said.contains("beat -8 of `chorus`"), "{said}");
}

#[test]
fn a_point_placed_before_the_piece_starts_is_refused() {
    let said = refusal(riding(
        verse_and_chorus(),
        vec![against("verse", -4.0, 0.1)],
    ));
    assert!(said.contains("beat -4 of `verse`"), "{said}");
    assert!(matches!(
        riding(verse_and_chorus(), vec![against("verse", -4.0, 0.1)]).validate(),
        Err(SynthError::BadAutomationSection { .. })
    ));
}

/// The section is the point of writing it, so it is what a tool writes back —
/// never the beat it happened to come to.
#[test]
fn a_written_section_survives_a_round_trip() {
    let song = riding(verse_and_chorus(), vec![against("chorus", -8.0, 0.1)]);
    let json = song.to_json().expect("it serialises");
    assert!(json.contains("\"section\": \"chorus\""), "{json}");
    assert!(json.contains("\"beat\": -8"), "{json}");
    assert_eq!(Song::from_json(&json).expect("it parses back"), song);

    let counted = riding(verse_and_chorus(), vec![at(8.0, 0.1)]);
    let json = counted.to_json().expect("it serialises");
    let points = &json[json.find("\"points\"").expect("a curve")..];
    assert!(!points.contains("section"), "{points}");
}
