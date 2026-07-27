//! The two claims the rest of scorsese is built on: a bake is **deterministic**,
//! and a note comes out at the **pitch it was asked for**.
//!
//! Everything else here is a detail of how a sound is made. These two are the
//! contract: `generated/` addresses a bake by the hash of its recipe, which is
//! only sound if the same recipe really does produce the same bytes.

#[path = "common/mod.rs"]
mod common;

use common::{minimal, opts, rising_crossings, saw_patch};
use scorsese_soundgen::patch::{Adsr, Osc, Patch, Source, Wave};
use scorsese_soundgen::{NoteOpts, bake_named_note, bake_note};

/// A bare sine with no envelope movement, so rising zero-crossings *are* the
/// frequency and the pitch can be asserted without an FFT.
fn sine() -> Patch {
    Patch {
        amp: Adsr {
            a: 0.0,
            d: 0.0,
            s: 1.0,
            r: 0.0,
        },
        ..minimal(Source::OscStack {
            oscs: vec![Osc {
                wave: Wave::Sine,
                detune_cents: 0.0,
                gain: 1.0,
                octave: 0,
            }],
        })
    }
}

#[test]
fn a_note_lands_on_the_pitch_it_was_asked_for() {
    let one_second = NoteOpts {
        duration: 1.0,
        velocity: 1.0,
        seed: 0,
    };
    let hz = |midi: f32| {
        let buf = scorsese_soundgen::render_note(&sine(), midi, &one_second).expect("renders");
        rising_crossings(&buf[..44_100])
    };
    assert_eq!(hz(69.0), 440, "A4 is 440 Hz");
    assert_eq!(hz(57.0), 220, "an octave down halves it");
    assert_eq!(hz(81.0), 880, "an octave up doubles it");
}

/// The claim `generated/` rests on. If this ever fails, a cache hit is serving
/// bytes the recipe no longer describes.
#[test]
fn the_same_recipe_bakes_the_same_bytes() {
    let patch = saw_patch();
    let first = bake_note(&patch, 60.0, &opts(0.3)).expect("bakes");
    let second = bake_note(&patch, 60.0, &opts(0.3)).expect("bakes");
    assert_eq!(
        first, second,
        "a bake must be a pure function of its inputs"
    );
}

/// A stochastic source is the interesting case: noise that is *seeded* still
/// has to be reproducible, and a different seed still has to differ.
#[test]
fn seeded_noise_is_reproducible_and_a_new_seed_is_not() {
    let noise = minimal(Source::Noise);
    let seeded =
        |seed: u64| bake_note(&noise, 60.0, &NoteOpts { seed, ..opts(0.2) }).expect("bakes");
    assert_eq!(seeded(11), seeded(11));
    assert_ne!(seeded(11), seeded(12));
}

#[test]
fn naming_a_note_and_numbering_it_bake_the_same_file() {
    let patch = saw_patch();
    let named = bake_named_note(&patch, "C#4", &opts(0.2)).expect("bakes by name");
    let numeric = bake_note(&patch, 61.0, &opts(0.2)).expect("bakes by number");
    assert_eq!(named, numeric);
}

#[test]
fn an_unknown_note_name_is_rejected_by_name() {
    let refused = bake_named_note(&saw_patch(), "H4", &opts(0.2)).expect_err("H is not a note");
    assert!(refused.to_string().contains("H4"), "got {refused}");
}
