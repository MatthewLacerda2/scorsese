//! Where an embedded clip does — and does not — cut the mix.

use scorsese_core::AssetKind;

use super::plan_of;
use crate::common::{
    audio_shape, audio_track, clip, file_asset, project, silent_asset, sounding_asset, video_track,
};

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
fn a_track_of_mixed_shots_contributes_only_the_ones_with_sound() {
    // The ordinary timeline: an interview, silent b-roll, another interview,
    // over a bed that changes clip halfway through the b-roll. Both halves of
    // the rule are here. The b-roll's own boundaries must not cut the mix — or
    // the bed would be decoded twice more for no reason — and the stretch that
    // *does* begin inside the b-roll, because the bed cut there, must not pick
    // it up as a layer.
    let project = project(
        vec![
            sounding_asset("interview", AssetKind::Video),
            silent_asset("broll", AssetKind::Video),
            file_asset("music", AssetKind::Audio),
        ],
        vec![
            video_track(
                "v1",
                vec![
                    clip("c1", "interview", 0, 30),
                    clip("c2", "broll", 40, 20),
                    clip("c3", "interview", 70, 20),
                ],
            ),
            audio_track(
                "a1",
                vec![clip("m1", "music", 0, 50), clip("m2", "music", 50, 40)],
            ),
        ],
    );
    let plan = plan_of(&project);

    assert_eq!(plan.segments().len(), 5, "three shots and two holes");
    assert_eq!(
        audio_shape(&plan),
        [
            (0, 30, "c1+m1".to_owned()),
            (30, 20, "m1".to_owned()),
            (50, 20, "m2".to_owned()),
            (70, 20, "c3+m2".to_owned()),
        ]
    );
}
