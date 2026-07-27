//! Where the stretches fall, and what they are called in each unit.
//!
//! The boundary is the whole value of the first half of a description: a shot
//! that starts a frame late is the mistake nobody sees and everybody ships. So
//! the times here are pinned exactly, in frames and in seconds, rather than to
//! anything near them.

use scorsese_core::{AssetKind, Fps, Frames};
use scorsese_render::FrameRange;

use crate::common::{audio_track, clip, file_asset, project, sounding_asset, video_track};
use crate::{described, described_over, shape};

/// Two shots meeting at frame 60, on a 30 fps grid: 2.00s exactly.
fn back_to_back() -> scorsese_core::Project {
    project(
        vec![
            file_asset("one", AssetKind::Video),
            file_asset("two", AssetKind::Video),
        ],
        vec![video_track(
            "v1",
            vec![clip("c1", "one", 0, 60), clip("c2", "two", 60, 30)],
        )],
    )
}

#[test]
fn a_cut_is_one_stretch_either_side_of_it() {
    let description = described(&back_to_back());

    assert_eq!(shape(&description), [(0, 60), (60, 90)]);
    assert_eq!(
        description.stretches[0].picture[0].clip, "c1",
        "the first shot fills the first stretch"
    );
    assert_eq!(description.stretches[1].picture[0].clip, "c2");
    assert_eq!(description.stretches[0].picture[0].track, "v1");
}

/// Frame 60 on a 30 fps grid is 2.00 seconds, and frame 90 is 3.00. If the
/// conversion were off by a frame in either direction every boundary in every
/// description would be, so this is asserted to the exact double rather than to
/// a tolerance.
#[test]
fn every_boundary_is_given_in_seconds_as_well_as_frames() {
    let description = described(&back_to_back());
    let (first, second) = (&description.stretches[0], &description.stretches[1]);

    assert_eq!((first.from.seconds, first.to.seconds), (0.0, 2.0));
    assert_eq!((second.from.seconds, second.to.seconds), (2.0, 3.0));
    assert_eq!(description.seconds(), 3.0);
}

/// The frame of the *file*, which is what a still is pulled from. With no range
/// it matches the timeline frame; the point is that it is derived rather than
/// assumed, so a render of part of a timeline still names frames of its own
/// output.
#[test]
fn each_stretch_knows_which_frame_of_the_file_it_starts_on() {
    assert_eq!(described(&back_to_back()).boundaries(), [0, 60]);

    let range = FrameRange::new(Frames(30), Some(Frames(90))).expect("a forward range");
    let ranged = described_over(&back_to_back(), range, Fps::THIRTY);
    assert_eq!(shape(&ranged), [(30, 60), (60, 90)]);
    assert_eq!(
        ranged.boundaries(),
        [0, 30],
        "the file starts where the range does, the timeline does not"
    );
    assert_eq!(
        (ranged.from.seconds, ranged.to.seconds),
        (1.0, 3.0),
        "seconds stay on the timeline, so they agree with the project"
    );
}

/// A hole between two clips is two seconds of black, not two seconds of missing
/// time. Describing it as anything else — or leaving it out — would make a cut
/// with a hole in it read exactly like a cut without one.
#[test]
fn a_hole_between_two_clips_is_described_as_a_gap() {
    let project = project(
        vec![
            file_asset("one", AssetKind::Video),
            file_asset("two", AssetKind::Video),
        ],
        vec![video_track(
            "v1",
            vec![clip("c1", "one", 0, 30), clip("c2", "two", 45, 30)],
        )],
    );

    let description = described(&project);

    assert_eq!(shape(&description), [(0, 30), (30, 45), (45, 75)]);
    assert!(!description.stretches[0].is_gap());
    assert!(
        description.stretches[1].is_gap(),
        "nothing is on screen between the two shots"
    );
    assert!(description.stretches[1].picture.is_empty());
    assert!(!description.stretches[2].is_gap());
    assert!(
        format!("{description}").contains("gap — black"),
        "and it is said in as many words:\n{description}"
    );
}

/// The mix cuts the timeline too. A shot that outlasts the narration under it
/// is two stretches, because at the second one the answer to "what is audible"
/// became different — and that is exactly the mistake ("the music stopped two
/// seconds early") a description is read to catch.
#[test]
fn the_mix_changing_cuts_a_stretch_the_picture_would_not_have() {
    let project = project(
        vec![
            file_asset("shot", AssetKind::Video),
            sounding_asset("music", AssetKind::Audio),
        ],
        vec![
            video_track("v1", vec![clip("c1", "shot", 0, 60)]),
            audio_track("a1", vec![clip("m1", "music", 0, 30)]),
        ],
    );

    let description = described(&project);

    assert_eq!(shape(&description), [(0, 30), (30, 60)]);
    assert_eq!(description.stretches[0].sound.len(), 1);
    assert!(
        description.stretches[1].sound.is_empty(),
        "the music ends halfway through the shot"
    );
    assert_eq!(description.stretches[1].picture[0].clip, "c1");
}
