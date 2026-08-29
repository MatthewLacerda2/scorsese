//! The published vocabulary, and what it says about a path nobody animates.

mod common;

use scorsese_compositor::{ANIMATED, Properties, Registry, path};
use scorsese_core::{ChromaKey, Frames, PropertyPath, Rgba};

use common::constant;

/// A registry of this crate's own properties, which is what the compositor
/// knows on its own. The real one also carries the mixer's `volume`.
const DRAWN: Registry = Registry::new(&[ANIMATED]);

fn path_of(text: &str) -> PropertyPath {
    PropertyPath::new(text)
}

#[test]
fn every_published_property_is_one_the_compositor_actually_resolves() {
    // The two halves that must not drift: a name here with no implementation is
    // a promise nothing keeps. Each published path is set to a value no default
    // has, so a path that resolved to nothing would leave the defaults standing.
    // `0.375` rather than a rounder number because `chroma_key.tolerance`
    // defaults to `0.25`, and a probe equal to the default proves nothing.
    // The baseline carries a key, because two of these paths are settings *of*
    // one: a `chroma_key.tolerance` track on a clip with no screen colour has
    // no key to be a tolerance of, and resolves to nothing for a reason that is
    // about the document rather than about the compositor. Every other property
    // is at its default here exactly as before.
    let baseline = Properties {
        chroma_key: Some(ChromaKey::new(Rgba::opaque(0, 177, 64))),
        ..Properties::default()
    };
    for property in ANIMATED {
        let animated = Properties::over(baseline, &[constant(property.path, 0.375)], Frames::ZERO);
        assert_ne!(
            animated, baseline,
            "`{}` is published but resolves to nothing",
            property.path
        );
        assert!(
            !property.describes.is_empty(),
            "`{}` says nothing about what it does",
            property.path
        );
    }
}

#[test]
fn every_property_the_compositor_resolves_is_published() {
    // And the other direction: an implemented property missing from the list is
    // one nothing can tell you about, so a project using it would be warned
    // about for no reason.
    for known in [
        path::OPACITY,
        path::POSITION_X,
        path::POSITION_Y,
        path::SCALE_X,
        path::SCALE_Y,
    ] {
        assert!(DRAWN.knows(&path_of(known)), "`{known}` is not published");
    }
}

#[test]
fn a_path_nobody_animates_is_not_known() {
    assert!(!DRAWN.knows(&path_of("opactiy")));
    // `transform.rotation` stood here, as the plausible-but-absent property.
    // It exists now, and that is exactly what this list is for: what is in it
    // is what this build draws, so the day that changes, this changes too.
    assert!(!DRAWN.knows(&path_of("transform.skew.x")));
    // `volume` really is unknown *to the compositor* — it is the mixer's, and
    // the union of the two is assembled where both are visible.
    assert!(!DRAWN.knows(&path_of("volume")));
}

#[test]
fn a_typo_is_told_what_it_probably_meant() {
    // The whole point of the feature: "`opactiy` is not a property" is half an
    // answer, and the half nobody needs.
    let suggestion = DRAWN
        .did_you_mean(&path_of("opactiy"))
        .expect("a transposition is a typo");
    assert_eq!(suggestion.path, path::OPACITY);

    let suggestion = DRAWN
        .did_you_mean(&path_of("transform.positon.x"))
        .expect("a dropped letter is a typo");
    assert_eq!(suggestion.path, path::POSITION_X);
}

#[test]
fn a_genuinely_novel_path_is_not_guessed_at() {
    // A project authored against a newer compositor is not making a typo, and a
    // confident wrong suggestion is worse than none: it is the one thing a
    // reader acts on without checking.
    assert_eq!(DRAWN.did_you_mean(&path_of("lighting.intensity")), None);
    assert_eq!(DRAWN.did_you_mean(&path_of("blur.radius")), None);
    assert_eq!(DRAWN.did_you_mean(&path_of("")), None);
}

#[test]
fn a_path_equally_close_to_two_properties_gets_no_suggestion() {
    // `transform.scale.z` is one edit from both `.x` and `.y` — and is exactly
    // what someone animating in three dimensions would write. Picking either
    // would be a coin flip presented as advice.
    assert_eq!(DRAWN.did_you_mean(&path_of("transform.scale.z")), None);
}
