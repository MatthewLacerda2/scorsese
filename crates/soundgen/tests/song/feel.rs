//! `swing` and `humanize`: a song played rather than clocked.

use crate::common::songs::{song, verse};
use crate::common::{peak, saw_patch};
use scorsese_soundgen::song::{Humanize, InlineOnly, PatchRef};
use scorsese_soundgen::{SAMPLE_RATE, Song, render_song};

/// Renders a song whose instruments are all inline.
fn render(song: &Song) -> Vec<f32> {
    render_song(song, &InlineOnly).expect("the song renders")
}

/// The fixture with a *deterministic* instrument.
///
/// The stock one is noise, which already differs from pass to pass by design
/// and would hide the thing these tests are about. With a plain saw, two
/// passes of one pattern are bit-identical unless something moved them apart.
fn steady() -> Song {
    let mut steady = song();
    steady.tracks[0].patch = PatchRef::Inline(Box::new(saw_patch()));
    steady
}

/// How many samples one pass of the two-beat fixture pattern takes.
fn pass(song: &Song) -> usize {
    (2.0 * song.beat_seconds() * SAMPLE_RATE as f32) as usize
}

/// The strongest form the claim takes: swinging a song equals writing its
/// off-beats late by hand, to the sample.
#[test]
fn swinging_equals_writing_the_off_beats_late() {
    let mut swung = song();
    verse(&mut swung).notes[1].start = 0.5;
    swung.swing = 0.5;

    let mut by_hand = song();
    verse(&mut by_hand).notes[1].start = 0.75;

    assert_eq!(render(&swung), render(&by_hand));
}

/// Swing moves the off-beats and nothing else: a note on the beat is left
/// where it was written, which is what makes swing a feel rather than a delay.
#[test]
fn a_note_on_the_beat_does_not_move_however_hard_the_song_swings() {
    let straight = song();
    let on_the_beat = straight.patterns["verse"]
        .notes
        .iter()
        .all(|note| note.start.fract() == 0.0);
    assert!(on_the_beat, "the fixture is written on the beat");

    let swung = Song {
        swing: 0.66,
        ..song()
    };
    assert_eq!(render(&swung), render(&straight));
}

/// The point of keying the draw on the note's ordinal rather than on its
/// pattern: eight bars nudged identically both times round is still a
/// photocopy, just a crooked one.
#[test]
fn a_pattern_played_twice_is_humanised_differently_each_time() {
    let rigid = render(&steady());
    let one = pass(&steady());
    assert_eq!(
        rigid[..one],
        rigid[one..2 * one],
        "the fixture must be identical pass to pass, or this proves nothing"
    );

    let played = render(&Song {
        humanize: Some(Humanize {
            timing: 0.02,
            velocity: 0.1,
        }),
        ..steady()
    });
    assert_ne!(
        played[..one],
        played[one..2 * one],
        "the second pass through a pattern must be played afresh"
    );
}

/// A note written on beat zero and nudged early has nowhere to go, so it is
/// clamped to the start of the buffer rather than lost off the front of it.
#[test]
fn an_early_nudge_on_the_very_first_beat_still_sounds() {
    let window = (0.6 * SAMPLE_RATE as f32) as usize;
    for seed in 0..8 {
        let wild = Song {
            seed,
            humanize: Some(Humanize {
                timing: 0.5,
                velocity: 0.0,
            }),
            ..steady()
        };
        let rendered = render(&wild);
        assert!(
            rendered.len() >= 2 * pass(&wild),
            "the song kept its length"
        );
        assert!(
            peak(&rendered[..window]) > 0.0,
            "seed {seed} lost the first note off the front"
        );
    }
}
