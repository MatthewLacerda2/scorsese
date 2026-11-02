//! The longest song a recipe may ask to be rendered: ten minutes, refused by
//! `validate` before a sample buffer is allocated. Every song here is checked,
//! never rendered: a song at the cap is ten minutes of samples.

use scorsese_zimmer::Song;
use scorsese_zimmer::song::{Fit, FitMode};

use crate::common::songs::{note, played, song};

/// The fixture at 60 bpm, its one pattern `beats` long: a beat a second.
fn lasting(beats: f32) -> Song {
    let mut song = Song {
        bpm: 60.0,
        ..song()
    };
    let verse = song
        .patterns
        .get_mut("verse")
        .expect("the fixture's pattern");
    verse.beats = beats;
    song.arrangement.truncate(1);
    song
}

#[test]
fn a_song_one_beat_past_ten_minutes_is_refused_and_one_at_them_is_not() {
    assert_eq!(lasting(600.0).validate(), Ok(()));
    let refused = lasting(601.0).validate().expect_err("past the cap");
    let said = refused.to_string();
    assert!(said.contains("601") && said.contains("600"), "{said}");
}

#[test]
fn a_loop_fit_cannot_ask_for_more_than_ten_minutes() {
    let mut fine = song();
    fine.fit = Some(Fit::lasting(600.0, FitMode::Loop));
    assert_eq!(fine.validate(), Ok(()));
    let mut long = song();
    long.fit = Some(Fit::lasting(1.0e6, FitMode::Loop));
    assert!(long.validate().is_err(), "a million seconds of loop");
}

#[test]
fn a_loop_cut_at_ten_minutes_is_not_counted_by_its_whole_passes() {
    // Seven seconds a pass: 86 passes are 602 s, cut to 600.
    let mut song = lasting(7.0);
    song.fit = Some(Fit::lasting(600.0, FitMode::Loop));
    assert_eq!(song.validate(), Ok(()));
}

#[test]
fn a_note_placed_far_past_its_pattern_counts_toward_the_length() {
    let mut song = lasting(4.0);
    let verse = song
        .patterns
        .get_mut("verse")
        .expect("the fixture's pattern");
    verse.notes = played(vec![note("bass", "E2", 1.0e6, 0.5)]);
    assert!(song.validate().is_err(), "a note a million beats in");
}
