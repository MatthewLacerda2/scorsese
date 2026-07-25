//! Several video tracks at once: where the timeline gets cut, and what is
//! visible through each piece.

use scorsese_core::Fps;
use scorsese_render::{FrameRange, Plan};

use crate::common::{clip, project, shape, video_track};
use crate::{overlaid, two_videos};

#[test]
fn tracks_stack_bottom_first_in_project_order() {
    let project = project(
        two_videos(),
        vec![
            video_track("v1", vec![clip("bed", "under", 0, 30)]),
            video_track("v2", vec![clip("top", "over", 0, 30)]),
        ],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");

    assert_eq!(
        shape(&plan),
        vec![(0, 30, "bed+top".to_owned(), 30)],
        "one stretch, two layers, the lower track first"
    );
    assert_eq!(plan.widest_stack(), 2);
}

#[test]
fn the_timeline_is_cut_where_any_track_starts_or_stops() {
    // The lower clip runs the whole length and is not cut by anything of its
    // own, but the upper one entering and leaving splits it into three.
    let project = overlaid();
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");

    assert_eq!(
        shape(&plan),
        vec![
            (0, 20, "bed".to_owned(), 20),
            (20, 20, "bed+top".to_owned(), 20),
            (40, 20, "bed".to_owned(), 20),
        ]
    );
    assert_eq!(plan.out_frames(), 60, "and it is still sixty frames long");
}

#[test]
fn a_hole_in_the_upper_track_leaves_the_lower_one_showing() {
    // The failure this guards against: contributing a black layer for the hole,
    // which would paint over the track below instead of letting it through.
    let project = project(
        two_videos(),
        vec![
            video_track("v1", vec![clip("bed", "under", 0, 60)]),
            video_track(
                "v2",
                vec![clip("first", "over", 0, 20), clip("second", "over", 40, 20)],
            ),
        ],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");

    assert_eq!(
        shape(&plan),
        vec![
            (0, 20, "bed+first".to_owned(), 20),
            (20, 20, "bed".to_owned(), 20),
            (40, 20, "bed+second".to_owned(), 20),
        ],
        "the middle stretch is one layer, not two with a black one on top"
    );
}

#[test]
fn a_stretch_with_nothing_on_any_track_is_still_a_gap() {
    let project = project(
        two_videos(),
        vec![
            video_track("v1", vec![clip("bed", "under", 0, 20)]),
            video_track("v2", vec![clip("top", "over", 40, 20)]),
        ],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");

    assert_eq!(
        shape(&plan),
        vec![
            (0, 20, "bed".to_owned(), 20),
            (20, 20, "gap".to_owned(), 20),
            (40, 20, "top".to_owned(), 20),
        ]
    );
    assert!(plan.segments()[1].is_gap());
}
