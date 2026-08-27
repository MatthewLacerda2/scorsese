//! Summing the notes: how long the result is, that it never clips, and that
//! the same song renders the same samples every time.

use crate::common::peak;
use crate::common::songs::{blip, note, played, song, verse, voice};
use scorsese_zimmer::song::{InlineOnly, PatchRef};
use scorsese_zimmer::{SAMPLE_RATE, Song, SynthError, render_song};

/// Renders a song whose instruments are all inline — every fixture here.
fn render(song: &Song) -> Vec<f32> {
    render_song(song, &InlineOnly).expect("the song renders")
}

/// Where the arrangement ends, in samples.
fn arrangement_end(song: &Song) -> usize {
    (song.arrangement_beats() * song.beat_seconds() * SAMPLE_RATE as f32).round() as usize
}

/// Length is `max(arrangement, last tail)`, and both halves matter for
/// different reasons: the arrangement floor keeps a song that ends on a rest
/// loopable, and the tail overhang keeps a final chord from being chopped
/// mid-ring.
#[test]
fn a_song_ending_on_a_rest_keeps_the_rest() {
    let song = song();
    assert_eq!(song.arrangement_beats(), 4.0, "two 2-beat patterns");
    // The last note is a short blip on beat 3 of 4, so its audio dies well
    // before the arrangement does.
    assert_eq!(
        render(&song).len(),
        arrangement_end(&song),
        "a song ending on a rest must still be a full arrangement long"
    );
}

#[test]
fn a_note_held_past_the_final_beat_rings_out_rather_than_being_cut() {
    let mut rings_out = song();
    voice(verse(&mut rings_out), 1).dur = 4.0;
    assert!(render(&rings_out).len() > arrangement_end(&rings_out));
}

/// Mixing is addition, which is exactly the operation that overshoots full
/// scale — so the master limiter is not optional.
#[test]
fn the_mix_never_clips() {
    let mut piled = song();
    verse(&mut piled).notes = played((0..12).map(|_| note("bass", "E2", 0.0, 1.0)).collect());
    piled.tracks[0].gain = 1.0;
    let peak = peak(&render(&piled));
    assert!(peak <= 1.0, "the master limiter let {peak} through");
}

#[test]
fn the_same_song_and_seed_render_identical_samples() {
    let seeded = song();
    assert_eq!(
        render(&seeded),
        render(&seeded),
        "the same song must render the same samples"
    );
}

/// The other half of the seeding contract: it has to actually re-roll, or the
/// seed is decoration.
#[test]
fn a_different_seed_re_rolls_the_stochastic_sources() {
    let seeded = song();
    let reseeded = Song {
        seed: 8,
        ..seeded.clone()
    };
    assert_ne!(render(&reseeded), render(&seeded));
}

/// A pattern played twice must not be a photocopy of itself — the note
/// ordinal, not just the seed, feeds each note's randomness.
#[test]
fn a_repeated_pattern_does_not_repeat_its_noise() {
    let song = song();
    let samples = render(&song);
    let half = arrangement_end(&song) / 2;
    assert_ne!(
        &samples[..half],
        &samples[half..half * 2],
        "the second pass through `verse` must not be identical to the first"
    );
}

#[test]
fn an_unresolvable_instrument_is_reported_with_its_track() {
    let mut missing = song();
    missing.tracks[0].patch = PatchRef::Named("recipes/nope.json".to_owned());
    let refused =
        render_song(&missing, &|_: &str| Err("no such recipe".to_owned())).expect_err("refused");
    assert_eq!(
        refused,
        SynthError::UnresolvedPatch {
            track: "bass".to_owned(),
            reference: "recipes/nope.json".to_owned(),
            reason: "no such recipe".to_owned(),
        }
    );
}

/// The resolver is the crate's only door to anything outside itself, so a
/// caller supplying one has to actually be used.
#[test]
fn a_named_instrument_is_taken_from_the_resolver() {
    let mut named = song();
    named.tracks[0].patch = PatchRef::Named("bass".to_owned());
    let resolved = render_song(&named, &|_: &str| Ok(blip())).expect("the resolver supplies it");
    assert_eq!(resolved, render(&song()), "same instrument, same samples");
}
