//! What a clip's speed changes about sequencing, and what it deliberately does
//! not.
//!
//! It changes exactly one thing: which point of the source each stretch of
//! timeline opens at. Where the cuts fall, how long the render is, how many
//! output frames a segment is worth — none of those know a clip has a rate, and
//! that is the property worth pinning, because a speed that moved a boundary
//! would move a cut nobody asked to move.

use scorsese_core::{AssetKind, Fps};
use scorsese_render::{FrameRange, Plan};

use crate::common::{
    audio_source_ins, audio_track, clip, clip_from, file_asset, project, shape, sounding_asset,
    source_ins, sped, video_track,
};
use crate::two_videos;

#[test]
fn a_sped_clip_walks_its_source_faster_than_the_timeline() {
    // Two segments, because the upper track cuts the lower one at 20 and 40.
    // At 2× the bed has consumed 40 source frames by timeline frame 20, and 80
    // by frame 40 — the arithmetic a puller assuming one-to-one gets wrong.
    let project = project(
        two_videos(),
        vec![
            video_track("v1", vec![sped(2.0, clip("bed", "under", 0, 60))]),
            video_track("v2", vec![clip("top", "over", 20, 20)]),
        ],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");

    assert_eq!(
        source_ins(&plan),
        vec![
            ("bed".to_owned(), 0.0),
            ("bed".to_owned(), 40.0),
            ("top".to_owned(), 0.0),
            ("bed".to_owned(), 80.0),
        ]
    );
}

/// Where the bed opens in each stretch an overlay cuts it into, with the bed
/// running at `rate` from `source_in`. Three answers, because the overlay
/// entering and leaving splits the timeline in three.
fn bed_opens(rate: f64, source_in: u64, duration: u64, over: (u64, u64)) -> Vec<f64> {
    let project = project(
        two_videos(),
        vec![
            video_track(
                "v1",
                vec![sped(
                    rate,
                    clip_from("bed", "under", 0, duration, source_in),
                )],
            ),
            video_track("v2", vec![clip("top", "over", over.0, over.1)]),
        ],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");
    source_ins(&plan)
        .into_iter()
        .filter(|(id, _)| id == "bed")
        .map(|(_, at)| at)
        .collect()
}

#[test]
fn a_slowed_clip_walks_it_slower() {
    assert_eq!(bed_opens(0.5, 0, 60, (20, 20)), vec![0.0, 10.0, 20.0]);
}

/// `source_in` is where playback begins and the rate only says how fast it
/// moves from there, so the two compose rather than one scaling the other.
#[test]
fn a_clips_own_source_in_is_where_the_rate_starts_from() {
    assert_eq!(
        bed_opens(3.0, 90, 60, (20, 20)),
        vec![90.0, 150.0, 210.0],
        "90, then three source frames per timeline frame"
    );
}

/// The whole reason the position is not a frame count. At 1.5× a stretch
/// opening 5 frames in opens 7.5 source frames in, and rounding that would
/// leave the resumed segment half a frame from where the last one stopped.
#[test]
fn a_rate_that_lands_between_frames_is_not_rounded_to_one() {
    assert_eq!(bed_opens(1.5, 0, 30, (5, 5)), vec![0.0, 7.5, 15.0]);
}

/// Segments are cut where the visible set changes, and a rate changes which
/// source frame is pulled rather than where a boundary is. A speed that moved
/// one would move a cut the author placed.
#[test]
fn a_rate_moves_no_boundary_and_shortens_no_render() {
    let tracks = |rate: f64| {
        vec![
            video_track("v1", vec![sped(rate, clip("bed", "under", 0, 60))]),
            video_track("v2", vec![clip("top", "over", 20, 20)]),
        ]
    };
    let ordinary = project(two_videos(), tracks(1.0));
    let quick = project(two_videos(), tracks(4.0));

    let plan = |project| {
        let plan = Plan::build(project, Fps::THIRTY, FrameRange::ALL).expect("plan");
        (shape(&plan), plan.out_frames())
    };
    assert_eq!(plan(&ordinary), plan(&quick));
}

/// One rate for picture and sound: the mix opens where the picture does, so a
/// sped shot stays in sync with itself.
#[test]
fn the_mix_walks_the_source_at_the_same_rate_as_the_picture() {
    let project = project(
        vec![
            sounding_asset("interview", AssetKind::Video),
            file_asset("music", AssetKind::Audio),
        ],
        vec![
            video_track("v1", vec![sped(2.0, clip("c1", "interview", 0, 60))]),
            audio_track("a1", vec![clip("m1", "music", 30, 30)]),
        ],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");

    assert_eq!(
        audio_source_ins(&plan),
        [
            ("c1".to_owned(), 0.0),
            ("c1".to_owned(), 60.0),
            ("m1".to_owned(), 0.0),
        ],
        "the sound is sped exactly as far as the picture is"
    );
}
