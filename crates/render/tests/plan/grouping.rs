//! A group clip in the plan: one layer, with the group's members beneath it,
//! cut wherever a member enters or leaves — on the group's own clock.

use scorsese_core::{Asset, AssetId, Fps, Frames, Group};
use scorsese_render::{FrameRange, Plan, PlanError};

use crate::common::{clip, clip_from, project, shape_asset, video_track};

/// A bed over the whole thirty frames, and a clip of a group placed at frame
/// 10 that opens five frames into it. Inside, `early` runs group frames 0–15
/// and `late` 12–30 — so on the timeline `late` arrives at 17 and `early`
/// leaves at 20.
fn placed() -> scorsese_core::Project {
    let group = Group::new(vec![
        video_track("g1", vec![clip("early", "box", 0, 15)]),
        video_track("g2", vec![clip("late", "box", 12, 18)]),
    ]);
    project(
        vec![
            shape_asset("box", 0.2, 0.2),
            Asset::group(AssetId::new("diagram"), group),
        ],
        vec![
            video_track("v1", vec![clip("bed", "box", 0, 30)]),
            video_track("v2", vec![clip_from("shown", "diagram", 10, 20, 5)]),
        ],
    )
}

#[test]
fn a_group_is_one_layer_cut_where_its_members_enter_and_leave() {
    let project = placed();
    project.validate().expect("valid");
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");

    let seen: Vec<(u64, Vec<&str>, Vec<&str>)> = plan
        .segments()
        .iter()
        .map(|segment| {
            let layers = segment.layers.iter().map(|s| s.clip.id.as_str()).collect();
            let members = segment
                .layers
                .iter()
                .flat_map(|s| s.members.iter().map(|m| m.clip.id.as_str()))
                .collect();
            (segment.start.get(), layers, members)
        })
        .collect();
    assert_eq!(
        seen,
        [
            (0, vec!["bed"], vec![]),
            (10, vec!["bed", "shown"], vec!["early"]),
            (17, vec!["bed", "shown"], vec!["early", "late"]),
            (20, vec!["bed", "shown"], vec!["late"]),
        ]
    );
    assert_eq!(
        plan.widest_stack(),
        4,
        "the bed, the group and both members"
    );
}

/// A member's own time is the group's: timeline frame 17 is group frame 12,
/// which is where `late` starts — so its keyframes count from there.
#[test]
fn a_member_keeps_the_groups_time() {
    let project = placed();
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");
    let segment = &plan.segments()[2];
    let late = &segment.layers[1].members[1];
    assert_eq!(late.local(Frames(17)), Frames(12));
    assert_eq!(late.local(Frames(29)), Frames(24));
}

/// Validation refuses it; a render of a document nobody validated refuses it
/// too, rather than opening the group forever.
#[test]
fn a_group_containing_itself_is_refused_not_recursed_into() {
    let mut project = placed();
    let group = project.assets[1].group.as_mut().expect("the group");
    group.tracks[0].clips.push(clip("again", "diagram", 20, 5));
    let error = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).unwrap_err();
    assert!(
        matches!(error, PlanError::GroupContainsItself { .. }),
        "got {error}"
    );
}
