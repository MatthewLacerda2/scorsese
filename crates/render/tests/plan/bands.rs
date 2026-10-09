//! Leaving the bands of unmade sounds off the picture (#966).
//!
//! A per-render choice, so it is pinned at the plan, where it acts: the clip
//! stays a clip in the mix and drops out of the picture, and the plan says
//! which lines it left out.

use scorsese_core::{AssetKind, Fps, Project};
use scorsese_render::{Bands, FrameRange, Note, Plan};

use crate::common::{
    audio_shape, audio_track, clip, file_asset, generated_asset, narration_asset, project, shape,
    video_track,
};

/// A shot with a narration line over its second half.
fn narrated(narration: scorsese_core::Asset) -> Project {
    project(
        vec![file_asset("a", AssetKind::Video), narration],
        vec![
            video_track("v1", vec![clip("shot", "a", 0, 60)]),
            audio_track("a1", vec![clip("vo1", "vo", 30, 30)]),
        ],
    )
}

fn planned(project: &Project, range: FrameRange, bands: Bands) -> Plan<'_> {
    Plan::drawing(project, Fps::THIRTY, range, bands).expect("the timeline sequences")
}

fn left_out(plan: &Plan<'_>) -> Vec<Note> {
    plan.notes()
        .iter()
        .filter(|note| matches!(note, Note::BandsLeftOut { .. }))
        .cloned()
        .collect()
}

#[test]
fn an_omitted_band_takes_the_line_off_the_picture_and_leaves_it_in_the_mix() {
    let project = narrated(narration_asset("vo"));
    let plan = planned(&project, FrameRange::ALL, Bands::Omitted);

    assert_eq!(
        shape(&plan),
        [(0, 60, "shot".to_owned(), 60)],
        "no band, and no cut where the band would have begun"
    );
    assert_eq!(
        audio_shape(&plan),
        [(0, 30, "silence".to_owned()), (30, 30, "vo1".to_owned())],
        "the line keeps its place in the mix"
    );
    assert_eq!(
        left_out(&plan),
        [Note::BandsLeftOut {
            clips: vec!["vo1".to_owned()]
        }]
    );
}

#[test]
fn drawn_is_what_build_has_always_done() {
    let project = narrated(narration_asset("vo"));
    let drawn = planned(&project, FrameRange::ALL, Bands::Drawn);
    let built = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("sequences");

    assert_eq!(shape(&drawn), shape(&built));
    assert_eq!(drawn.notes(), built.notes());
    assert!(left_out(&drawn).is_empty());
    assert_eq!(
        shape(&drawn),
        [
            (0, 30, "shot".to_owned(), 30),
            (30, 30, "shot+vo1".to_owned(), 30)
        ]
    );
}

#[test]
fn a_generated_line_has_no_band_to_leave_out() {
    let project = narrated(generated_asset("vo", AssetKind::GeneratedAudio));
    let plan = planned(&project, FrameRange::ALL, Bands::Omitted);

    assert!(left_out(&plan).is_empty(), "nothing was left out");
}

/// The note is about this render: a line outside the range asked for was
/// never going to be on it.
#[test]
fn only_the_lines_inside_the_range_are_named() {
    let project = narrated(narration_asset("vo"));
    let plan = planned(&project, "0:30".parse().expect("a range"), Bands::Omitted);

    assert!(left_out(&plan).is_empty());
    let plan = planned(&project, "59:60".parse().expect("a range"), Bands::Omitted);
    assert_eq!(
        left_out(&plan).len(),
        1,
        "a frame of the line is a frame of it"
    );
}
