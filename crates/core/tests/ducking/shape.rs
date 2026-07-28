//! Where the points land.

use crate::{dip, points, scored};
use scorsese_core::{PropertyPath, TrackId, Under, duck_track};

/// Ducks the fixture's music track and hands back its points.
fn duck(narration: &[(u64, u64)]) -> Vec<(u64, f64)> {
    let mut project = scored(narration);
    let under = Under {
        music: TrackId::new("music"),
        narration: vec![],
    };
    duck_track(&mut project, &under, &PropertyPath::new("volume"), dip())
        .expect("the music track exists");
    points(&project)
}

/// The four points of one dip, and the one that matters most: the level is
/// **fully down by the frame the narration starts**, not on its way there.
#[test]
fn one_line_gets_four_points_and_is_down_before_it_speaks() {
    // Narration over frames 60..150, attack 9, release 18.
    assert_eq!(
        duck(&[(60, 90)]),
        vec![(51, 1.0), (60, 0.25), (150, 0.25), (168, 1.0)]
    );
}

/// Two lines far apart are two dips: the music comes all the way back between
/// them, which is the point of ducking rather than just turning it down.
#[test]
fn two_distant_lines_are_two_dips() {
    let points = duck(&[(60, 60), (400, 60)]);
    assert_eq!(points.len(), 8);
    assert_eq!(points[3], (138, 1.0), "back to full after the first");
    assert_eq!(
        points[4],
        (391, 1.0),
        "and still full going into the second"
    );
}

/// Two lines close together are **one** dip. Coming up and immediately going
/// back down is pumping, which draws more attention than the ducking avoided.
#[test]
fn two_close_lines_are_one_dip() {
    // A 20-frame gap, against a 27-frame release-plus-attack.
    let points = duck(&[(60, 90), (170, 60)]);
    assert_eq!(
        points,
        vec![(51, 1.0), (60, 0.25), (230, 0.25), (248, 1.0)],
        "the gap is not worth surfacing for"
    );
}

/// Overlapping narration is obviously one dip, and must not double up.
#[test]
fn overlapping_lines_are_one_dip() {
    let points = duck(&[(60, 120), (100, 120)]);
    assert_eq!(points.len(), 4);
    assert_eq!(points[2].0, 220, "the dip ends with the later of the two");
}

/// A dip cannot start before the clip does, and the attack has nowhere to go.
#[test]
fn narration_at_the_very_start_clamps_rather_than_underflowing() {
    let points = duck(&[(0, 60)]);
    assert_eq!(points[0], (0, 0.25), "already down at frame zero");
    assert_eq!(points.len(), 3);
}

/// A dip cannot run past the end of the clip either, and a release with no
/// room simply lands on the last frame.
#[test]
fn narration_at_the_very_end_clamps_to_the_clip() {
    let points = duck(&[(560, 60)]);
    assert!(
        points.iter().all(|(frame, _)| *frame <= 600),
        "no point past the clip's 600 frames: {points:?}"
    );
}

#[test]
fn no_narration_writes_no_track() {
    assert_eq!(duck(&[]), vec![], "silence over it is not a dip of nothing");
}

/// Ducking must never emit a track validation would reject — repeated or
/// out-of-order times are the realistic way that could happen.
#[test]
fn every_dip_it_writes_is_a_legal_track() {
    for narration in [
        vec![(60, 90)],
        vec![(0, 60)],
        vec![(60, 90), (170, 60)],
        vec![(60, 60), (400, 60)],
        vec![(560, 60)],
    ] {
        let points = duck(&narration);
        let times: Vec<u64> = points.iter().map(|(frame, _)| *frame).collect();
        let mut sorted = times.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(times, sorted, "times must ascend strictly: {points:?}");
    }
}
