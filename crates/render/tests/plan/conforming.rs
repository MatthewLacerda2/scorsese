//! Which timeline instant an output frame draws when the two rates differ.
//!
//! Above the timeline's rate a segment is worth more output frames than
//! timeline frames, and the conform back rounds to nearest: unclamped, the
//! last of them could land on the segment's `end`, which is the next clip's
//! first frame and the outgoing one's out point (#1092).

use scorsese_core::{AssetKind, Clip, Fps, Frames, Project};
use scorsese_render::{FrameRange, Plan};

use crate::common::{file_asset, project, video_track};

/// One-frame clips back to back, so every timeline frame is its own segment
/// and every segment end is a place to land on.
fn single_frames(timeline_fps: Fps) -> Project {
    let clips = (0..24)
        .map(|frame| Clip::at(&format!("c{frame}"), "a", frame, 1))
        .collect();
    Project {
        timeline_fps,
        ..project(
            vec![file_asset("a", AssetKind::Video)],
            vec![video_track("v1", clips)],
        )
    }
}

#[test]
fn every_output_frame_draws_an_instant_inside_its_own_segment() {
    let rate = |num, den| Fps::new(num, den).expect("a frame rate");
    for (timeline, out) in [
        (rate(24, 1), Fps::THIRTY),
        (rate(24, 1), rate(60, 1)),
        (rate(25, 1), Fps::NTSC),
        (Fps::THIRTY, rate(24, 1)),
    ] {
        let project = single_frames(timeline);
        let plan = Plan::build(&project, out, FrameRange::ALL).expect("plan");
        for segment in plan.segments() {
            for index in 0..plan.out_frames_of(segment) {
                let at = plan.timeline_frame_of(segment, index);
                assert!(
                    segment.start <= at && at < segment.end(),
                    "{timeline:?} -> {out:?}: output frame {index} of [{:?}, {:?}) drew {at:?}",
                    segment.start,
                    segment.end(),
                );
            }
        }
    }
}

#[test]
fn the_last_frame_of_a_24_fps_clip_is_drawn_inside_it_at_30() {
    // The issue's own case: a clip at [1, 2) of a 24 fps timeline, then a
    // cut. At 30 fps its segment is two output frames, and the second used to
    // conform to 1 + round(0.8) = 2 — the clip's out point, where a
    // `fade_out` has already reached nothing, a frame before the cut.
    let project = Project {
        timeline_fps: Fps::new(24, 1).expect("24 fps"),
        ..project(
            vec![file_asset("a", AssetKind::Video)],
            vec![video_track(
                "v1",
                vec![
                    Clip::at("lead", "a", 0, 1),
                    Clip::at("out", "a", 1, 1),
                    Clip::at("next", "a", 2, 24),
                ],
            )],
        )
    };
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");
    let out = &plan.segments()[1];
    assert_eq!((out.start, out.end()), (Frames(1), Frames(2)));
    assert_eq!(plan.out_frames_of(out), 2);
    assert_eq!(plan.timeline_frame_of(out, 1), Frames(1));
}
