//! Where each layer opens in its own source, once other tracks start cutting
//! the timeline underneath it.

use scorsese_core::{AssetKind, Fps};
use scorsese_render::{FrameRange, Plan};

use crate::common::{
    audio_track, clip, clip_from, file_asset, project, shape, sounding_asset, source_ins,
    video_track,
};
use crate::{overlaid, two_videos};

#[test]
fn a_clip_cut_by_another_track_resumes_at_the_right_source_frame() {
    // `bed` appears in three segments. Each has to open where the last left
    // off, or the picture jumps back to the start of the clip at every cut on
    // an unrelated track.
    let project = overlaid();
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");

    assert_eq!(
        source_ins(&plan),
        vec![
            ("bed".to_owned(), 0),
            ("bed".to_owned(), 20),
            ("top".to_owned(), 0),
            ("bed".to_owned(), 40),
        ]
    );
}

#[test]
fn a_clips_own_source_in_still_counts_when_another_track_cuts_it() {
    let project = project(
        two_videos(),
        vec![
            video_track("v1", vec![clip_from("bed", "under", 0, 60, 100)]),
            video_track("v2", vec![clip("top", "over", 20, 20)]),
        ],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");

    let beds: Vec<u64> = source_ins(&plan)
        .into_iter()
        .filter(|(id, _)| id == "bed")
        .map(|(_, at)| at)
        .collect();
    assert_eq!(beds, vec![100, 120, 140], "the clip's own offset is added");
}

#[test]
fn a_range_trims_a_stack_the_same_way_it_trims_one_track() {
    let project = overlaid();
    let range: FrameRange = "25:45".parse().expect("a range");
    let plan = Plan::build(&project, Fps::THIRTY, range).expect("plan");

    assert_eq!(
        shape(&plan),
        vec![
            (25, 15, "bed+top".to_owned(), 15),
            (40, 5, "bed".to_owned(), 5),
        ]
    );
    assert_eq!(
        source_ins(&plan),
        vec![
            ("bed".to_owned(), 25),
            ("top".to_owned(), 5),
            ("bed".to_owned(), 40),
        ],
        "both layers open where the range starts, each in its own clip's terms"
    );
}

#[test]
fn a_shot_resumes_its_own_sound_across_a_cut_on_another_track() {
    // The same rule that keeps a music bed from restarting: where a stretch
    // opens in a clip's source is the clip's business, not the segment's.
    let project = project(
        vec![
            sounding_asset("interview", AssetKind::Video),
            file_asset("music", AssetKind::Audio),
        ],
        vec![
            video_track("v1", vec![clip("c1", "interview", 0, 60)]),
            audio_track("a1", vec![clip("m1", "music", 30, 30)]),
        ],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");

    let opens: Vec<(String, u64)> = plan
        .audio()
        .iter()
        .flat_map(|segment| segment.layers.iter())
        .map(|shot| (shot.clip.id.to_string(), shot.source_in.get()))
        .collect();
    assert_eq!(
        opens,
        [
            ("c1".to_owned(), 0),
            ("c1".to_owned(), 30),
            ("m1".to_owned(), 0),
        ],
        "the shot picks its sound up where it left off"
    );
}
