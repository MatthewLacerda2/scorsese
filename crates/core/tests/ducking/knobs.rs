//! The shape knobs: where the level rests between lines, and how long a
//! pause has to be before the music breathes back up in it.

use crate::{dip, points, scored};
use scorsese_core::{Dip, Easing, PropertyPath, TrackId, Under, duck_track};

fn duck(narration: &[(u64, u64)], dip: Dip) -> scorsese_core::Project {
    let mut project = scored(narration);
    let under = Under::new(TrackId::new("music"));
    duck_track(&mut project, &under, &PropertyPath::new("volume"), dip).expect("ducks");
    project
}

/// The Stories ad's lines as `(start, duration)`, ~0.2 to 0.4 s apart.
const STORIES: [(u64, u64); 4] = [(4, 62), (77, 110), (199, 127), (332, 64)];

/// `between` is where the music rests when nobody is speaking.
#[test]
fn between_is_the_level_outside_the_lines() {
    let ducked = duck(&[(60, 90)], Dip { over: 0.5, ..dip() });
    assert_eq!(
        points(&ducked),
        vec![(51, 0.5), (60, 0.25), (150, 0.25), (168, 0.5)]
    );
}

/// A short `min_gap` keeps the Stories ad's pauses. Each is shorter than
/// the 27 frames of release and attack, so it is shared between them and the
/// level touches full once on its way back down.
#[test]
fn a_short_min_gap_breathes_in_pauses_the_default_merges() {
    let ducked = duck(&STORIES, dip().with_min_gap_seconds(0.1, 30.0));
    #[rustfmt::skip]
    let expected = vec![
        (0, 1.0), (4, 0.25), (66, 0.25),
        (73, 1.0), (77, 0.25), (187, 0.25),
        (195, 1.0), (199, 0.25), (326, 0.25),
        (330, 1.0), (332, 0.25), (396, 0.25), (414, 1.0),
    ];
    assert_eq!(points(&ducked), expected);
    let peak = &ducked.tracks[0].clips[0].keyframes[0].keyframes[3];
    assert_eq!(
        peak.easing,
        Easing::EaseOut,
        "the peak starts the next fall"
    );
}

/// The three knobs together come close to the curve the Stories ad's music was
/// given by hand: 0.25 under, 0.55 in the pauses, quick ramps. Where a pause is
/// exactly both ramps long (330..332) the climb and the fall meet on a frame.
#[test]
fn the_knobs_together_reproduce_the_stories_ad_by_hand() {
    let shaped = Dip {
        over: 0.55,
        ..Dip::from_seconds(0.25, 0.1, 0.1, 30.0).with_min_gap_seconds(0.1, 30.0)
    };
    #[rustfmt::skip]
    let expected = vec![
        (1, 0.55), (4, 0.25), (66, 0.25), (69, 0.55),
        (74, 0.55), (77, 0.25), (187, 0.25), (190, 0.55),
        (196, 0.55), (199, 0.25), (326, 0.25), (329, 0.55),
        (332, 0.25), (396, 0.25), (399, 0.55),
    ];
    assert_eq!(points(&duck(&STORIES, shaped)), expected);
}

/// A long `min_gap` holds two distant lines under as one.
#[test]
fn a_long_min_gap_merges_pauses_the_default_keeps() {
    let lines = [(60, 60), (400, 60)];
    assert_eq!(points(&duck(&lines, dip())).len(), 8, "two dips by default");
    let held = duck(&lines, dip().with_min_gap_seconds(10.0, 30.0));
    assert_eq!(
        points(&held),
        vec![(51, 1.0), (60, 0.25), (460, 0.25), (478, 1.0)]
    );
}

/// A ramp of no length is drawn over one frame. Before, its two points landed
/// on one frame and one was dropped: the level glided down across the whole
/// line, or never came back up after it.
#[test]
fn a_zero_ramp_is_a_one_frame_step() {
    let switch = Dip::from_seconds(0.25, 0.0, 0.0, 30.0);
    assert_eq!(
        points(&duck(&[(60, 90), (400, 50)], switch)),
        vec![
            (59, 1.0),
            (60, 0.25),
            (150, 0.25),
            (151, 1.0),
            (399, 1.0),
            (400, 0.25),
            (450, 0.25),
            (451, 1.0)
        ]
    );
}

/// Whatever the pause and whatever `min_gap`, the track is one validation
/// accepts: times strictly ascending, and every pause kept has a peak in it.
#[test]
fn every_pause_length_writes_a_legal_track() {
    for gap in 0..40 {
        for min_gap in [0.0, 0.1, 0.5] {
            let ducked = duck(
                &[(60, 90), (150 + gap, 60)],
                dip().with_min_gap_seconds(min_gap, 30.0),
            );
            let times: Vec<u64> = points(&ducked).iter().map(|(t, _)| *t).collect();
            assert!(
                times.windows(2).all(|pair| pair[0] < pair[1]),
                "gap {gap}, min_gap {min_gap}: {times:?}"
            );
        }
    }
}
