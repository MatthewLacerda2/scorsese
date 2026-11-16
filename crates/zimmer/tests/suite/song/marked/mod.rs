//! Section markers (#1792): `{ "section": "chorus" }` in the arrangement names
//! the run of entries after it. The claim nearly every test here holds to:
//! **a marker changes how a song is reported, and nothing it plays.**

mod anchored;
mod written;

use scorsese_zimmer::song::{ArrangementEntry, InlineOnly, Marker};
use scorsese_zimmer::{Song, render_song};

/// Six two-beat entries at 120 bpm — a second each — on a noise track and a
/// pitched one, so a note whose noise was keyed differently would show.
const UNMARKED: &str = r#"{
  "bpm": 120, "seed": 7,
  "tracks": [
    { "name": "hat", "patch": { "source": { "kind": "noise" },
        "amp": { "a": 0.001, "d": 0.05, "s": 0.0, "r": 0.02 } } },
    { "name": "bass", "patch": { "source": { "kind": "osc_stack", "oscs": [{ "wave": "saw" }] },
        "amp": { "a": 0.01, "d": 0.1, "s": 0.8, "r": 0.1 } } }
  ],
  "patterns": {
    "a": { "beats": 2, "notes": [
      { "track": "hat", "note": "C4", "start": 0, "dur": 0.25 },
      { "track": "bass", "note": "E2", "start": 0, "dur": 1.5 } ] },
    "b": { "beats": 2, "notes": [
      { "track": "hat", "note": "C4", "start": 1, "dur": 0.25 } ] }
  },
  "arrangement": ["a", "b", "a", "b", "a", { "layers": ["a", "b"] }],
  "tail": "exact"
}"#;

pub(crate) fn unmarked() -> Song {
    Song::from_json(UNMARKED).expect("the fixture parses")
}

pub(crate) fn marker(name: &str) -> ArrangementEntry {
    Marker {
        section: name.to_owned(),
    }
    .into()
}

/// The fixture with `markers` inserted: each `(index, name)` goes in before
/// the entry at that index of the unmarked arrangement.
pub(crate) fn marked(markers: &[(usize, &str)]) -> Song {
    let mut song = unmarked();
    for &(index, name) in markers.iter().rev() {
        song.arrangement.insert(index, marker(name));
    }
    song
}

fn rows(song: &Song) -> Vec<(String, f64)> {
    song.sections()
        .into_iter()
        .map(|cut| (cut.label, cut.end_seconds))
        .collect()
}

pub(crate) fn render(song: &Song) -> Vec<f32> {
    render_song(song, &InlineOnly).expect("the song renders")
}

/// Markers take no slot and are no part of the seed walk: the same song
/// marked up renders to the same samples, noise included.
#[test]
fn a_marked_song_renders_byte_identical_to_the_unmarked_one() {
    let song = marked(&[(0, "intro"), (2, "verse"), (4, "chorus")]);
    song.validate().expect("valid");
    assert_eq!(song.arrangement_beats(), unmarked().arrangement_beats());
    assert_eq!(render(&song), render(&unmarked()));
}

/// A song with no marker is reported as it always was, one row per entry.
#[test]
fn an_unmarked_song_has_a_row_per_entry() {
    let labels: Vec<String> = rows(&unmarked()).into_iter().map(|row| row.0).collect();
    assert_eq!(labels, ["a", "b", "a", "b", "a", "a + b"]);
}

/// Each named section is one row, ending where its last entry ends; an entry
/// before the first marker keeps a row of its own.
#[test]
fn each_named_section_is_one_row_ending_with_its_last_entry() {
    let song = marked(&[(1, "verse"), (4, "chorus")]);
    song.validate().expect("valid");
    assert_eq!(
        rows(&song),
        [
            ("a".to_owned(), 1.0),
            ("verse".to_owned(), 4.0),
            ("chorus".to_owned(), 6.0),
        ]
    );
}

/// A loop plays the arrangement again, so its rows come round again.
#[test]
fn a_loop_repeats_the_named_rows_per_pass() {
    let mut song = marked(&[(0, "verse"), (3, "chorus")]);
    song.fit = Some(scorsese_zimmer::song::Fit::lasting(
        12.0,
        scorsese_zimmer::song::FitMode::Loop,
    ));
    let labels: Vec<String> = rows(&song).into_iter().map(|row| row.0).collect();
    assert_eq!(labels, ["verse", "chorus", "verse", "chorus"]);
}
