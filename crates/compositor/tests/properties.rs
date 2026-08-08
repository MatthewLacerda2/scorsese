//! Resolving property paths into the properties a layer draws with.
//!
//! This is where `scorsese-core`'s opaque property strings acquire meaning, and
//! the reason it is here and not there: adding an animatable property must cost
//! a change next to the code that implements it, and nothing to the format.

mod common;

use scorsese_compositor::{Properties, path};
use scorsese_core::{Frames, Grade};

use common::constant;

#[test]
fn a_clip_that_animates_nothing_composites_as_it_arrived() {
    assert_eq!(
        Properties::over(Grade::NEUTRAL, &[], Frames(7)),
        Properties::default()
    );
    assert!(Properties::default().is_identity());
}

#[test]
fn every_known_path_reaches_its_property() {
    let tracks = vec![
        constant(path::OPACITY, 0.25),
        constant(path::POSITION_X, 0.1),
        constant(path::POSITION_Y, -0.04),
        constant(path::SCALE_X, 2.0),
        constant(path::SCALE_Y, 0.5),
        constant(path::ROTATION, 15.0),
        constant(path::FLIP_X, 30.0),
        constant(path::FLIP_Y, 45.0),
    ];
    let properties = Properties::over(Grade::NEUTRAL, &tracks, Frames::ZERO);
    assert_eq!(properties.opacity, 0.25);
    assert_eq!(properties.position, (0.1, -0.04));
    assert_eq!(properties.scale, (2.0, 0.5));
    assert_eq!(properties.rotation, 15.0);
    assert_eq!(properties.flip, (30.0, 45.0));
}

#[test]
fn an_unknown_path_is_ignored_rather_than_fatal() {
    // A project authored against a newer compositor has to still render on an
    // older one. The cost is that a typo does nothing quietly, which is why
    // warning about unknown paths is its own piece of work.
    let tracks = vec![constant("opactiy", 0.1), constant(path::OPACITY, 0.5)];
    let properties = Properties::over(Grade::NEUTRAL, &tracks, Frames::ZERO);
    assert_eq!(properties.opacity, 0.5, "the real path still lands");
}

#[test]
fn a_layer_that_would_draw_nothing_is_recognised() {
    let invisible = [
        Properties {
            opacity: 0.0,
            ..Properties::default()
        },
        Properties {
            scale: (0.0, 1.0),
            ..Properties::default()
        },
        Properties {
            scale: (1.0, 0.0),
            ..Properties::default()
        },
        // Edge on, about either axis: the cosine is zero, so there is no width
        // or no height left to draw, and the rasteriser never sees it.
        Properties {
            flip: (0.0, 90.0),
            ..Properties::default()
        },
        Properties {
            flip: (-90.0, 0.0),
            ..Properties::default()
        },
    ];
    for properties in invisible {
        assert!(properties.is_invisible(), "{properties:?}");
    }
    assert!(!Properties::default().is_invisible());
}

#[test]
fn a_moved_or_faded_layer_is_not_an_identity() {
    let transformed = [
        Properties {
            position: (1.0, 0.0),
            ..Properties::default()
        },
        Properties {
            scale: (1.5, 1.0),
            ..Properties::default()
        },
        Properties {
            opacity: 0.9,
            ..Properties::default()
        },
        // A mirrored layer draws its own pixels, but not in their own places.
        Properties {
            flip: (0.0, 180.0),
            ..Properties::default()
        },
    ];
    for properties in transformed {
        assert!(!properties.is_identity(), "{properties:?}");
    }
}
