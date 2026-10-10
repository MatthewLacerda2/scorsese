//! The effects are starting points to vary (#1001), so what is held here is
//! the part of each description a number can check: where it sits, which
//! knobs move it, and that a texture lasts as long as it is held.

use super::*;

const EFFECTS: &[&str] = &[
    "whoosh", "riser", "pop", "click", "ding", "key", "scribble", "page", "thud",
];

fn bake(name: &str, note: &str, duration: f32) -> scorsese_zimmer::Bake {
    let opts = NoteOpts {
        duration,
        ..NoteOpts::default()
    };
    bake_named_note(&lookup(name).expect(name).patch(), note, &opts)
        .unwrap_or_else(|error| panic!("{name} does not render: {error}"))
}

#[test]
fn every_effect_is_in_the_kit_and_says_what_to_vary() {
    for name in EFFECTS {
        let effect = lookup(name).unwrap_or_else(|| panic!("{name} is not in the kit"));
        assert!(effect.describes.contains("Vary"), "{}", effect.describes);
    }
}

#[test]
fn each_effect_sits_where_its_name_says() {
    let bands = |name: &str| bands_of(lookup(name).expect(name));
    assert!(bands("thud").0 > 70, "a thud is low: {:?}", bands("thud"));
    for name in ["whoosh", "pop", "ding", "key", "page"] {
        assert!(
            bands(name).1 > 60,
            "{name} is mostly mid: {:?}",
            bands(name)
        );
    }
    for name in ["scribble", "riser"] {
        assert!(bands(name).2 > 25, "{name} is bright: {:?}", bands(name));
    }
}

/// A description that says noise ignores `note` is telling an agent not to
/// reach for that knob, so it had better be true — and one that offers
/// `note` had better mean it.
#[test]
fn note_moves_exactly_the_effects_that_say_it_does() {
    for name in EFFECTS {
        let effect = lookup(name).expect(name);
        let moved = bake(name, effect.home, 0.3).wav != bake(name, "G#3", 0.3).wav;
        let ignores = effect.describes.contains("ignores `note`");
        assert_eq!(moved, !ignores, "{name}: {}", effect.describes);
    }
}

/// A pencil drawing a stroke for four seconds needs four seconds of pencil.
#[test]
fn a_scribble_lasts_as_long_as_it_is_held() {
    let seconds = |held: f32| {
        let wav = bake("scribble", "C4", held).wav;
        (wav.len() - 44) as f32 / 4.0 / scorsese_zimmer::SAMPLE_RATE as f32
    };
    let (short, long) = (seconds(1.0), seconds(4.0));
    assert!(
        (long - short - 3.0).abs() < 0.1,
        "{short}s held 1, {long}s held 4"
    );
}
