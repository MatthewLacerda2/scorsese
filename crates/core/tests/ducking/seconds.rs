//! Building a dip from seconds, the way the CLI and MCP both do.

use scorsese_core::{Dip, Frames};

/// The defaults are the constants, put on the grid.
#[test]
fn the_default_is_the_constants_on_the_grid() {
    let dip = Dip::default_for(30.0);
    assert_eq!(dip.under, Dip::UNDER);
    assert_eq!(dip.over, 1.0);
    assert_eq!(dip.attack, Frames(9));
    assert_eq!(dip.release, Frames(18));
}

/// A ramp asked for at all is at least a frame: rounded to zero it would be a
/// switch. Zero, a negative and NaN are no ramp.
#[test]
fn a_ramp_is_a_frame_at_least_and_nothing_asked_is_none() {
    let tiny = Dip::from_seconds(0.5, 0.001, 0.001, 30.0);
    assert_eq!((tiny.attack, tiny.release), (Frames(1), Frames(1)));
    for nothing in [0.0, -1.0, f64::NAN] {
        let dip = Dip::from_seconds(0.5, nothing, nothing, 30.0);
        assert_eq!(
            (dip.attack, dip.release),
            (Frames::ZERO, Frames::ZERO),
            "{nothing}"
        );
    }
}
