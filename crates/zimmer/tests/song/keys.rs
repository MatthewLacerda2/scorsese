//! A key, degrees of it, and the lift that only a key makes expressible.
//!
//! The assertions are the same shape the chord tests take: **writing a degree
//! must equal writing the note it names.** The instrument is a plucked string
//! on purpose — pitched, so a wrong note fails, and *stochastic*, so
//! byte-equality also says the degree took the ordinal, and therefore the
//! seed, that the spelled note would have taken.

use crate::common::minimal;
use crate::common::songs::{note, played, song, verse};
use scorsese_zimmer::patch::Source;
use scorsese_zimmer::song::{
    ArrangementEntry, Degree, DegreeNote, InlineOnly, PatchRef, PatternEntry, Play,
};
use scorsese_zimmer::{Song, SynthError, render_song};

/// Renders a song whose instruments are all inline.
fn render(song: &Song) -> Vec<f32> {
    render_song(song, &InlineOnly).expect("the song renders")
}

/// The fixture in `key`, playing `entries` once through.
fn playing(key: Option<&str>, entries: Vec<PatternEntry>) -> Song {
    let mut one_pass = song();
    one_pass.arrangement = vec!["verse".into()];
    one_pass.key = key.map(str::to_owned);
    one_pass.tracks[0].patch = PatchRef::Inline(Box::new(minimal(Source::Karplus {
        damping: 0.99,
        brightness: 0.5,
    })));
    verse(&mut one_pass).notes = entries;
    one_pass
}

/// One degree on the fixture's `bass` track, half a beat long.
fn degree(written: Degree, oct: i32, start: f32) -> PatternEntry {
    DegreeNote {
        track: "bass".to_owned(),
        degree: written,
        oct: Some(oct),
        start,
        dur: 0.5,
        vel: 1.0,
    }
    .into()
}

/// The fixture's one pattern, played with the transposes given.
fn lifted(song: &mut Song, transpose: Option<f32>, degrees: Option<i32>) {
    song.arrangement = vec![ArrangementEntry::Transformed(Play {
        pattern: "verse".to_owned(),
        transpose,
        transpose_degrees: degrees,
        vel_scale: None,
        tracks: None,
    })];
}

/// The three notes of a D minor triad, by name, at half a beat each.
fn triad(names: [&str; 3]) -> Vec<PatternEntry> {
    played(
        names
            .iter()
            .enumerate()
            .map(|(index, name)| note("bass", name, index as f32 * 0.5, 0.5))
            .collect(),
    )
}

/// The numbering rule, end to end: `1 3 5` in D minor is D F A, not D F# A.
#[test]
fn a_degree_renders_as_the_note_it_names_in_the_key() {
    let degrees = playing(
        Some("D minor"),
        vec![
            degree(Degree::Plain(1), 4, 0.0),
            degree(Degree::Plain(3), 4, 0.5),
            degree(Degree::Plain(5), 4, 1.0),
        ],
    );
    assert_eq!(
        render(&degrees),
        render(&playing(Some("D minor"), triad(["D4", "F4", "A4"])))
    );
}

/// The alteration grammar, which is what keeps a minor key's leading tone and
/// every borrowed note writable as a degree at all.
#[test]
fn an_altered_degree_is_the_accidental_it_writes() {
    let sharpened = playing(
        Some("D minor"),
        vec![degree(Degree::Altered("#7".to_owned()), 4, 0.0)],
    );
    let spelled = playing(Some("D minor"), played(vec![note("bass", "C#5", 0.0, 0.5)]));
    assert_eq!(render(&sharpened), render(&spelled));
}

/// The lift: up one step **within** the key, which moves the three notes of
/// the triad by different numbers of semitones — the thing one chromatic
/// number cannot express, and the reason this field exists.
#[test]
fn a_diatonic_transpose_moves_within_the_key() {
    let mut lift = playing(Some("D minor"), triad(["D4", "E4", "F4"]));
    lifted(&mut lift, None, Some(1));
    let by_hand = playing(Some("D minor"), triad(["E4", "F4", "G4"]));
    assert_eq!(render(&lift), render(&by_hand));

    // And the chromatic transpose that is nearest to it is a different piece:
    // two semitones takes the F to F#, which is out of the key.
    let mut chromatic = playing(Some("D minor"), triad(["D4", "E4", "F4"]));
    lifted(&mut chromatic, Some(2.0), None);
    assert_ne!(render(&chromatic), render(&lift));
}

/// Everything a key is needed for is refused when the song declares none, and
/// the two lifts are refused together — see `transpose_degrees`.
#[test]
fn what_needs_a_key_is_refused_rather_than_guessed() {
    let refusal = |song: &Song| song.validate().expect_err("the song is refused");
    let keyless = playing(None, vec![degree(Degree::Plain(5), 4, 0.0)]);
    assert!(matches!(
        refusal(&keyless),
        SynthError::DegreeWithoutKey { .. }
    ));

    let mut no_key = playing(None, triad(["D4", "E4", "F4"]));
    lifted(&mut no_key, None, Some(1));
    assert!(matches!(
        refusal(&no_key),
        SynthError::DiatonicWithoutKey { .. }
    ));

    let mut both = playing(Some("D minor"), triad(["D4", "E4", "F4"]));
    lifted(&mut both, Some(12.0), Some(1));
    assert!(matches!(refusal(&both), SynthError::TwoTransposes { .. }));

    let unreadable = playing(Some("D harmonic minor"), triad(["D4", "E4", "F4"]));
    assert!(matches!(refusal(&unreadable), SynthError::BadKey { .. }));
}

/// Absolute names are untouched by a key being declared — including a note
/// that is *not* in it, because a deliberate accidental is a real thing.
#[test]
fn declaring_a_key_changes_nothing_a_song_already_said() {
    let notes = triad(["C#4", "F4", "A4"]);
    assert_eq!(
        render(&playing(Some("D minor"), notes.clone())),
        render(&playing(None, notes))
    );
}

/// The document keeps the key as it was spelled rather than as a pitch class,
/// and an entry that is two kinds at once is refused rather than read as one.
#[test]
fn a_key_and_a_degree_round_trip_as_they_were_written() {
    let written = playing(
        Some("Db minor"),
        vec![degree(Degree::Altered("b3".to_owned()), 3, 0.0)],
    );
    let json = written.to_json().expect("the song serialises");
    assert!(json.contains(r#""key": "Db minor""#), "{json}");
    assert_eq!(Song::from_json(&json).expect("it parses back"), written);

    let both = json.replace(r#""degree": "b3""#, r#""degree": 3, "note": "C4""#);
    assert!(Song::from_json(&both).is_err(), "{both}");
}
