//! The keyframes a dissolve writes, and the signature that keeps them its own.

use scorsese_compositor::DissolveError;
use scorsese_compositor::dissolve::{TOOL, dissolve};
use scorsese_core::{ClipId, Frames};

use crate::{clip, cut, opacity};

#[test]
fn a_dissolve_is_four_ordinary_keyframes() {
    let mut project = cut();
    dissolve(
        &mut project,
        &ClipId::new("a"),
        &ClipId::new("b"),
        Frames(10),
    )
    .expect("two shots that meet");

    // The outgoing shot goes out exactly on its own cut, not a frame before —
    // the same shape `fade_out` writes, for the same reason.
    assert_eq!(opacity(&project, "a"), [(20, 1.0), (30, 0.0)]);
    // The incoming one is solid by the time the outgoing one has gone.
    assert_eq!(opacity(&project, "b"), [(0, 0.0), (10, 1.0)]);
}

/// The signature earning its keep. A second run cannot stack a second
/// crossover on the first: the shots no longer meet, so it refuses, and the
/// keyframes already written are left exactly as they were.
#[test]
fn running_it_twice_leaves_one_dissolve() {
    let mut project = cut();
    let (a, b) = (ClipId::new("a"), ClipId::new("b"));
    dissolve(&mut project, &a, &b, Frames(10)).expect("the first");

    let again = dissolve(&mut project, &a, &b, Frames(10));

    assert!(matches!(again, Err(DissolveError::DoNotMeet { .. })));
    assert_eq!(opacity(&project, "b"), [(0, 0.0), (10, 1.0)]);
    assert_eq!(
        clip(&project, "b")
            .keyframes
            .iter()
            .filter(|track| track.by.as_deref() == Some(TOOL))
            .count(),
        1,
        "one dissolve track, not two"
    );
}
