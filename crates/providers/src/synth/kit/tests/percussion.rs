//! The metal and the percussion past the drum machine's seven (#1776): a
//! cymbal is a cluster of inharmonic tones, not hiss with an envelope, and a
//! groove has a ride, a rim, a shaker, a tambourine and a cowbell to reach for.

use super::*;
use scorsese_zimmer::patch::Source;

/// Struck metal and wood ring at pitches, however many and however far off
/// the harmonic grid; beads and jingle wash do not. So the head of each is
/// the kind of sound it is, and a hat that went back to filtered noise would
/// be told apart from its open self and the crash by length alone again.
#[test]
fn what_rings_leads_with_a_tone_and_what_rattles_with_noise() {
    let head_is_noise = |name: &str| {
        matches!(
            lookup(name).expect(name).patch().source,
            Source::Noise { .. }
        )
    };
    for name in [
        "hat",
        "open_hat",
        "crash",
        "ride",
        "rimshot",
        "tambourine",
        "cowbell",
    ] {
        assert!(!head_is_noise(name), "{name} is struck: its head is a tone");
    }
    assert!(head_is_noise("shaker"), "a shaker is beads, so noise");
}

/// A ride is a ping over a wash and a tambourine a jingle over a hit: two
/// sounds in one note, which is what a layer is.
#[test]
fn the_ride_and_the_tambourine_are_two_sounds_at_once() {
    for name in ["ride", "tambourine", "hat", "open_hat", "crash"] {
        let patch = lookup(name).expect(name).patch();
        assert!(
            patch
                .layers
                .iter()
                .any(|layer| matches!(layer.source, Source::Noise { .. })),
            "{name} carries a wash of noise under its tones"
        );
    }
}

#[test]
fn each_percussion_sits_where_its_name_says() {
    let bands = |name: &str| bands_of(lookup(name).expect(name));
    let (_, ride_mid, ride_high) = bands("ride");
    assert!(
        ride_mid > 30 && ride_high > 30,
        "a ride's ping is mid and its wash high: {:?}",
        bands("ride")
    );
    assert!(bands("rimshot").1 > 80, "rimshot: {:?}", bands("rimshot"));
    assert!(bands("cowbell").1 > 90, "cowbell: {:?}", bands("cowbell"));
    assert!(bands("shaker").2 > 75, "shaker: {:?}", bands("shaker"));
    assert!(
        bands("tambourine").2 > 70,
        "tambourine: {:?}",
        bands("tambourine")
    );
}
