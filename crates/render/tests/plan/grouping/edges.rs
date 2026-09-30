//! A group's members at the edges of the window its clip shows, and the
//! members whose sound a group leaves out of the mix.

use scorsese_core::{Asset, AssetId, AssetKind, Fps, Group, Project};
use scorsese_render::{FrameRange, Note, Plan};

use crate::common::{clip, clip_from, project, shape_asset, sounding_asset, video_track};

/// What each segment starts at and which members it opens.
fn members_by_segment(project: &Project) -> Vec<(u64, Vec<String>)> {
    let plan = Plan::build(project, Fps::THIRTY, FrameRange::ALL).expect("plan");
    plan.segments()
        .iter()
        .map(|segment| {
            let members = segment.layers.iter().flat_map(|s| &s.members);
            let ids = members.map(|m| m.clip.id.to_string()).collect();
            (segment.start.get(), ids)
        })
        .collect()
}

/// A clip of a group at timeline 10 opening five frames in, so it shows group
/// frames 5–25. One member starts exactly where the window opens and one ends
/// exactly where it closes: each edge is the group clip's own edge, so neither
/// adds a cut, and the one change inside is at 20, where the first hands over
/// to the second.
#[test]
fn a_member_edge_on_the_windows_edge_is_the_group_clips_own_cut() {
    let group = Group::new(vec![
        video_track("g1", vec![clip("opens", "box", 5, 10)]),
        video_track("g2", vec![clip("closes", "box", 15, 10)]),
    ]);
    let project = project(
        vec![
            shape_asset("box", 0.2, 0.2),
            Asset::group(AssetId::new("diagram"), group),
        ],
        vec![
            video_track("v1", vec![clip("bed", "box", 0, 30)]),
            video_track("v2", vec![clip_from("shown", "diagram", 10, 20, 5)]),
        ],
    );
    project.validate().expect("valid");
    let seen = members_by_segment(&project);
    let expected: Vec<(u64, Vec<String>)> = vec![
        (0, vec![]),
        (10, vec!["opens".into()]),
        (20, vec!["closes".into()]),
    ];
    assert_eq!(seen, expected);
}

/// A member showing a file with sound on it is drawn and not heard, and the
/// plan says which member of which group. A silent member, and a sounding one
/// in a group nothing places, are not mentioned.
#[test]
fn a_placed_groups_sounding_member_is_named_in_a_note() {
    let placed = Group::new(vec![
        video_track("g1", vec![clip("talk", "camera", 0, 10)]),
        video_track("g2", vec![clip("frame", "box", 0, 10)]),
    ]);
    let spare = Group::new(vec![video_track(
        "g3",
        vec![clip("unused", "camera", 0, 10)],
    )]);
    let project = project(
        vec![
            shape_asset("box", 0.2, 0.2),
            sounding_asset("camera", AssetKind::Video),
            Asset::group(AssetId::new("diagram"), placed),
            Asset::group(AssetId::new("spare"), spare),
        ],
        vec![video_track("v1", vec![clip("shown", "diagram", 0, 10)])],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");
    assert_eq!(
        plan.notes(),
        [Note::GroupedSoundLeftOut {
            clip: "talk".into(),
            group: "diagram".into(),
        }]
    );
}
