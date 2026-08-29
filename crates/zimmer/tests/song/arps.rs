//! An arpeggio as one entry, held to the notes it replaced.
//!
//! The claim is the chord tests' claim one level down: **writing a figure must
//! equal writing its notes out by hand.** The failure this file exists to catch
//! is silent — a permutation that walks the voices in some other order still
//! renders perfectly good music, it is simply not the music the page describes,
//! and nothing but an ear would find it.

use crate::common::songs::{note, played, song, verse};
use scorsese_zimmer::song::{Arp, Chord, Humanize, InlineOnly, Note, PatternEntry, Voicing};
use scorsese_zimmer::{Song, SynthError, render_song};

/// The one-track fixture over four beats, playing `entries` once and quietly
/// enough that the master limiter never acts.
fn playing(entries: Vec<PatternEntry>) -> Song {
    let mut once = song();
    once.arrangement = vec!["verse".into()];
    once.tracks[0].gain = 0.15;
    let verse = verse(&mut once);
    verse.beats = 4.0;
    verse.notes = entries;
    once
}

/// A four-beat chord on the fixture's track, arpeggiated as the fields say.
fn arped(name: &str, arp: Option<Arp>, div: Option<f32>, gate: Option<f32>) -> Chord {
    Chord {
        track: "bass".to_owned(),
        chord: Voicing::Name(name.to_owned()),
        oct: Some(3),
        start: 0.0,
        dur: 4.0,
        vel: 1.0,
        articulation: None,
        arp,
        div,
        gate,
    }
}

/// The same figure written out one note at a time, on the same grid — the
/// pitches in the order they sound, which is how the page prints them too.
fn by_hand(pitches: &str, div: f32, gate: f32) -> Vec<Note> {
    pitches
        .split_whitespace()
        .enumerate()
        .map(|(step, name)| note("bass", name, step as f32 * div, gate))
        .collect()
}

/// Rendered samples, which is where the two spellings have to agree.
fn render(song: &Song) -> Vec<f32> {
    render_song(song, &InlineOnly).expect("the song renders")
}

/// That the shorthand is the long form. The fixture's instrument is
/// *stochastic* on purpose: matching here means every note of the figure took
/// the ordinal it would have taken written out, noise seed and all.
fn same(chord: Chord, pitches: &str, div: f32, gate: f32) {
    assert_eq!(
        render(&playing(vec![chord.into()])),
        render(&playing(played(by_hand(pitches, div, gate)))),
        "the figure did not play the notes it stands for"
    );
}

/// Two traversals of a seventh chord, which is what four beats of eighths is.
#[test]
fn an_arpeggio_equals_writing_its_notes_out_by_hand() {
    let two = "D3 F3 A3 C4 D3 F3 A3 C4";
    same(arped("Dm7", Some(Arp::Up), Some(0.5), None), two, 0.5, 0.5);
}

/// Three voices over eight steps: the last traversal is cut where the chord
/// ends. Refusing that would be the step string's grid rule applied where there
/// is no written count to check it against — see the module doc on `arp`.
#[test]
fn a_figure_repeats_until_the_chord_is_over_and_truncates_there() {
    let cut = "E3 G3 B3 E3 G3 B3 E3 G3";
    same(arped("Em", Some(Arp::Up), Some(0.5), None), cut, 0.5, 0.5);
}

/// The other two words, stated as the pitches they play — including the turn
/// that plays neither end twice.
#[test]
fn down_and_up_down_walk_the_voices_the_page_says_they_do() {
    same(
        arped("Dm7", Some(Arp::Down), Some(1.0), None),
        "C4 A3 F3 D3",
        1.0,
        1.0,
    );
    let turn = "D3 F3 A3 C4 A3 F3 D3 F3";
    same(
        arped("Dm7", Some(Arp::UpDown), Some(0.5), None),
        turn,
        0.5,
        0.5,
    );
}

/// A chord exactly one traversal long plays its voices once and stops, which is
/// the strum — and the reason there is no fourth word for it.
#[test]
fn a_chord_the_length_of_one_traversal_plays_it_once() {
    let strum = Chord {
        dur: 2.0,
        ..arped("Dm7", Some(Arp::Up), Some(0.5), None)
    };
    same(strum, "D3 F3 A3 C4", 0.5, 0.5);
}

/// One step unless the chord writes one, so a figure on a sustaining patch can
/// accumulate into the chord it came from.
#[test]
fn a_gate_is_one_step_unless_the_chord_writes_one() {
    same(
        arped("Dm7", Some(Arp::Up), Some(1.0), Some(2.0)),
        "D3 F3 A3 C4",
        1.0,
        2.0,
    );
}

/// Expansion happens before the performance, so a figure swings and is
/// humanised note by note exactly as the notes it stands for are.
#[test]
fn every_note_of_a_figure_is_played_rather_than_clocked() {
    let feel = Humanize {
        timing: 0.4,
        velocity: 0.3,
        timbre: 0.2,
    };
    let scattered = |song: Song| {
        render(&Song {
            humanize: Some(feel),
            swing: 0.3,
            ..song
        })
    };
    let voices = "D3 F3 A3 C4 D3 F3 A3 C4";
    let figure = arped("Dm7", Some(Arp::Up), Some(0.5), None);
    assert_eq!(
        scattered(playing(vec![figure.into()])),
        scattered(playing(played(by_hand(voices, 0.5, 0.5))))
    );
}

/// The words round-trip, and a block chord writes none of the three fields — a
/// bake is addressed by the bytes of its recipe, so a default written into
/// every chord would miss the cache for every song in every project.
#[test]
fn an_arpeggio_round_trips_and_a_block_chord_says_nothing() {
    let figure = playing(vec![
        arped("Dm7", Some(Arp::UpDown), Some(0.5), Some(0.4)).into(),
    ]);
    let json = figure.to_json().expect("the song serialises");
    assert!(json.contains("\"arp\": \"up_down\""), "{json}");
    assert_eq!(Song::from_json(&json).expect("reads back"), figure);

    let block = playing(vec![arped("Dm7", None, None, None).into()]);
    let json = block.to_json().expect("the song serialises");
    for field in ["\"arp\"", "\"div\"", "\"gate\""] {
        assert!(!json.contains(field), "{field} in {json}");
    }
}

/// The fields an arpeggio needs, and the ones it forbids. A `div` on a chord
/// that is not arpeggiated is refused rather than ignored, for the reason `oct`
/// beside spelled pitches is: silence is the worst answer to a sentence the
/// writer believes they wrote.
#[test]
fn what_an_arpeggio_is_refused_for() {
    let refused = |chord: Chord, why: &str| {
        let refusal = playing(vec![chord.into()]).validate();
        assert!(
            matches!(refusal, Err(SynthError::BadArp { .. })),
            "{why}: {refusal:?}"
        );
    };
    let up = |div, gate| arped("Dm7", Some(Arp::Up), div, gate);
    refused(up(None, None), "an arp with no step");
    refused(arped("Dm7", None, Some(0.5), None), "a step with no arp");
    refused(arped("Dm7", None, None, Some(0.5)), "a gate with no arp");
    refused(up(Some(8.0), None), "a step longer than the chord");
    refused(up(Some(0.0), None), "a step that is not a length");
    refused(up(Some(0.5), Some(-1.0)), "a gate that is not a length");
    refused(up(Some(0.0001), None), "a figure past the cap");
    assert!(
        playing(vec![up(Some(0.5), Some(2.0)).into()])
            .validate()
            .is_ok(),
        "the figure the page documents must still be legal"
    );
}
