//! A group's members are clips like any other: every per-clip edit reaches
//! them, and the one edit that cannot leave the group's own time says so by
//! naming the group (#1103, #1102).

use scorsese_core::grouping::{self, Grouped};
use scorsese_core::placing::{relocate, remove, trim};
use scorsese_core::{
    ClipId, Frames, Level, Project, PropertyPath, RelocateError, Relocation, TrackId, Trim, level,
};

use super::{choose, diagram, ids};

/// The diagram with `b1`, `b2` and the arrow wrapped into a group, `b1` noted.
fn grouped() -> (Project, Grouped) {
    let mut project = diagram();
    project.tracks[1].clips[0].note = Some("the first box".to_owned());
    let grouped = grouping::group(&mut project, &choose(&["b1", "b2", "arrow"])).expect("accepted");
    (project, grouped)
}

fn b1() -> ClipId {
    ClipId::new("b1")
}

#[test]
fn a_member_is_found_by_id() {
    let (mut project, _) = grouped();
    assert!(project.clip(&b1()).is_some());
    assert!(project.clip_mut(&b1()).is_some());
    assert_eq!(
        project.group_holding(&b1()).map(|id| id.as_str()),
        Some("group")
    );
    assert_eq!(project.group_holding(&ClipId::new("bg")), None);
}

#[test]
fn a_note_on_a_member_is_read() {
    let (project, _) = grouped();
    assert!(project.notes().any(|noted| noted.id == "b1"));
}

#[test]
fn a_member_takes_a_level() {
    let (mut project, _) = grouped();
    let opacity = PropertyPath::new("opacity");
    level::set(&mut project, &b1(), &opacity, Level::Flat(0.5)).expect("accepted");
    assert_eq!(project.clip(&b1()).expect("accepted").keyframes.len(), 1);
}

#[test]
fn a_member_is_trimmed_in_place() {
    let (mut project, _) = grouped();
    let bounds = Trim {
        duration: Some(Frames(20)),
        ..Trim::default()
    };
    trim(&mut project, &b1(), &bounds).expect("accepted");
    assert_eq!(project.clip(&b1()).expect("accepted").duration, Frames(20));
}

#[test]
fn a_member_is_removed_and_the_group_keeps_the_rest() {
    let (mut project, _) = grouped();
    remove(&mut project, &ids(&["b1"])).expect("accepted");
    assert!(project.clip(&b1()).is_none());
    assert!(project.clip(&ClipId::new("b2")).is_some());
}

#[test]
fn a_member_moves_between_its_own_groups_tracks() {
    let (mut project, grouped) = grouped();
    let onto = Relocation {
        track: grouped.tracks[2].clone(),
        bounds: Trim {
            start: Some(Frames(50)),
            ..Trim::default()
        },
    };
    relocate(&mut project, &b1(), &onto).expect("accepted");
    let lane = project.every_track().find(|t| t.id == grouped.tracks[2]);
    assert!(
        lane.expect("accepted")
            .clips
            .iter()
            .any(|clip| clip.id == b1())
    );
}

#[test]
fn a_member_moved_onto_the_timeline_is_refused_by_naming_its_group() {
    let (mut project, _) = grouped();
    let onto = Relocation {
        track: TrackId::new("v1"),
        bounds: Trim::default(),
    };
    let error = relocate(&mut project, &b1(), &onto).expect_err("refused");
    assert!(matches!(error, RelocateError::Across(_)));
    assert!(
        error.to_string().contains("`b1` is inside group `group`"),
        "{error}"
    );
}
