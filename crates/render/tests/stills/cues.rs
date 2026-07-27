//! Asking for one moment rather than every cut.

use scorsese_core::Frames;
use scorsese_render::Cue;

use crate::Rendered;

/// Named instants, in either unit, land on the frames a description would have
/// named — and a moment asked for twice is decoded once.
#[test]
fn stills_can_be_asked_for_by_time_or_by_frame() {
    let fixture = Rendered::new("stills-cues");

    let wanted: Vec<u64> = [Cue::Seconds(1.0), Cue::Frame(Frames(29)), Cue::Seconds(1.0)]
        .iter()
        .map(|cue| cue.out_frame(&fixture.description))
        .collect();
    assert_eq!(wanted, [30, 29, 30]);

    let written = fixture.write(&wanted);
    let indices: Vec<u64> = written.iter().map(|still| still.frame).collect();
    assert_eq!(
        indices,
        [29, 30],
        "asked twice for one instant, written once"
    );

    assert_eq!(fixture.colour(&written[0]), "red");
    assert_eq!(fixture.colour(&written[1]), "blue");
}
