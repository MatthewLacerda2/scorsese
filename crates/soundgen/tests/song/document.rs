//! Writing a song down: the round trip, how a pitch may be spelled, and what
//! the renderer refuses before it starts.

use crate::common::songs::{song, verse};
use scorsese_soundgen::SynthError;
use scorsese_soundgen::song::{Pitch, Song};

#[test]
fn the_document_round_trips_through_json() {
    let original = song();
    let json = original.to_json().expect("serialise");
    assert_eq!(
        Song::from_json(&json).expect("deserialise"),
        original,
        "a song must survive a save/load round trip"
    );
}

/// Untagged serde has to route a JSON string to the name arm and a number to
/// MIDI. If those ever raced, a song would change pitch on being reloaded.
#[test]
fn a_pitch_may_be_a_name_or_a_midi_number() {
    assert_eq!(
        Pitch::Name("C4".to_owned()).to_midi().expect("a real note"),
        60.0
    );
    assert_eq!(Pitch::Midi(61.5).to_midi().expect("microtonal"), 61.5);
    assert!(Pitch::Name("H4".to_owned()).to_midi().is_err());

    let from_name: Pitch = serde_json::from_str(r#""C#4""#).expect("a string is a name");
    let from_midi: Pitch = serde_json::from_str("61").expect("a number is a MIDI value");
    assert_eq!(from_name.to_midi().expect("parses"), 61.0);
    assert_eq!(from_midi.to_midi().expect("parses"), 61.0);
}

#[test]
fn a_tempo_of_zero_is_refused() {
    let song = Song { bpm: 0.0, ..song() };
    assert_eq!(song.validate(), Err(SynthError::BadBpm { bpm: 0.0 }));
}

/// The failure this catches is *silence in the middle of a piece*, which is
/// the one an agent would lose a whole iteration noticing.
#[test]
fn an_arrangement_naming_a_pattern_that_does_not_exist_is_refused() {
    let mut song = song();
    song.arrangement = vec!["chorus".into()];
    assert_eq!(
        song.validate(),
        Err(SynthError::UnknownPattern {
            pattern: "chorus".to_owned()
        })
    );
}

#[test]
fn a_note_on_a_track_that_does_not_exist_is_refused_by_position() {
    let mut song = song();
    verse(&mut song).notes[1].track = "lead".to_owned();
    assert_eq!(
        song.validate(),
        Err(SynthError::UnknownTrack {
            pattern: "verse".to_owned(),
            index: 1,
            track: "lead".to_owned(),
        })
    );
}

#[test]
fn a_note_held_for_no_time_is_refused() {
    let mut song = song();
    verse(&mut song).notes[0].dur = 0.0;
    assert_eq!(
        song.validate(),
        Err(SynthError::BadNoteDuration {
            pattern: "verse".to_owned(),
            index: 0,
            dur: 0.0,
        })
    );
}

#[test]
fn a_song_with_nothing_to_play_is_refused() {
    let mut empty = song();
    empty.arrangement.clear();
    assert_eq!(empty.validate(), Err(SynthError::EmptyArrangement));

    let mut silent = song();
    silent.tracks.clear();
    assert_eq!(silent.validate(), Err(SynthError::NoTracks));
}
