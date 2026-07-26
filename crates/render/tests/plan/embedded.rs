//! Which clips on a *video* track are also heard.

use scorsese_core::{AssetKind, Fps};
use scorsese_render::{FrameRange, Plan};

use crate::common::{
    audio_shape, audio_track, clip, file_asset, project, silent_asset, sounding_asset, video_track,
};

fn plan_of(project: &scorsese_core::Project) -> Plan<'_> {
    Plan::build(project, Fps::THIRTY, FrameRange::ALL).expect("plan")
}

#[test]
fn a_video_clip_with_sound_on_it_is_mixed() {
    let project = project(
        vec![sounding_asset("interview", AssetKind::Video)],
        vec![video_track("v1", vec![clip("c1", "interview", 0, 30)])],
    );

    assert_eq!(
        audio_shape(&plan_of(&project)),
        [(0, 30, "c1".to_owned())],
        "a shot with dialogue on it belongs in the mix"
    );
}

#[test]
fn a_video_clip_with_no_sound_on_it_is_not() {
    // And the mix is not merely silent, it does not exist: a film shot without
    // sound gets a file with no audio stream, which is not the same as one with
    // a stream of silence.
    let project = project(
        vec![silent_asset("plate", AssetKind::Video)],
        vec![video_track("v1", vec![clip("c1", "plate", 0, 30)])],
    );

    assert!(plan_of(&project).audio().is_empty());
}

#[test]
fn an_unprobed_asset_is_not_taken_for_audible() {
    // The plan reads the assets table and nothing else. "Nobody asked" is not
    // "yes", and the render's own probe pass is what turns the first into an
    // answer before the plan ever sees it.
    let project = project(
        vec![file_asset("unknown", AssetKind::Video)],
        vec![video_track("v1", vec![clip("c1", "unknown", 0, 30)])],
    );

    assert!(plan_of(&project).audio().is_empty());
}

#[test]
fn a_silent_shot_does_not_cut_the_bed_it_sits_over() {
    // The picture cuts three times; the music does not. A clip that contributes
    // no sound must not split the mix, or a bed would be decoded once per shot
    // for no reason and every seam would be a chance to drift.
    let project = project(
        vec![
            silent_asset("plate", AssetKind::Video),
            file_asset("music", AssetKind::Audio),
        ],
        vec![
            video_track(
                "v1",
                vec![
                    clip("c1", "plate", 0, 30),
                    clip("c2", "plate", 30, 30),
                    clip("c3", "plate", 60, 30),
                ],
            ),
            audio_track("a1", vec![clip("m1", "music", 0, 90)]),
        ],
    );
    let plan = plan_of(&project);

    assert_eq!(plan.segments().len(), 3, "three shots");
    assert_eq!(audio_shape(&plan), [(0, 90, "m1".to_owned())], "one bed");
}

#[test]
fn an_audible_shot_cuts_the_mix_where_its_own_clip_starts_and_ends() {
    // The other side of it: a shot that *is* heard is one more audible thing on
    // the timeline, so its boundaries are boundaries of the mix like any audio
    // clip's, and the bed under it is summed with it in the middle stretch.
    let project = project(
        vec![
            silent_asset("plate", AssetKind::Video),
            sounding_asset("interview", AssetKind::Video),
            file_asset("music", AssetKind::Audio),
        ],
        vec![
            // A silent plate runs the whole length, so the picture — which is
            // what decides how long the render is — outlasts everything.
            video_track("v1", vec![clip("c0", "plate", 0, 90)]),
            video_track("v2", vec![clip("c1", "interview", 30, 30)]),
            audio_track("a1", vec![clip("m1", "music", 0, 90)]),
        ],
    );
    let plan = plan_of(&project);

    assert_eq!(
        audio_shape(&plan),
        [
            (0, 30, "m1".to_owned()),
            (30, 30, "c1+m1".to_owned()),
            (60, 30, "m1".to_owned()),
        ]
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
    let plan = plan_of(&project);

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
