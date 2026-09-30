//! Ungrouping: the inverse, and the two cases it refuses rather than guesses.

use scorsese_core::grouping::{self, UngroupError};
use scorsese_core::{ClipId, Frames};

use crate::{choose, diagram};

#[test]
fn ungrouping_puts_every_member_back_where_it_was() {
    let original = diagram();
    let mut project = original.clone();
    let grouped = grouping::group(&mut project, &choose(&["b1", "b2", "arrow"])).unwrap();
    let back = grouping::ungroup(&mut project, &grouped.clip).unwrap();

    assert_eq!(back.clips, 3);
    assert!(!back.dropped_its_own_look);
    assert!(project.asset(&grouped.asset).is_none(), "the group is gone");
    for (_, before) in original.clips() {
        let (_, after) = project
            .clips()
            .find(|(_, c)| c.id == before.id)
            .expect("every clip is back on the timeline");
        assert_eq!(
            (after.start, after.duration),
            (before.start, before.duration)
        );
    }
    let lanes: Vec<&str> = project.tracks.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(
        lanes,
        ["v1", "v2", "group-v2", "group-v3", "group-v4", "v3", "v4"],
        "the group's tracks, directly above where its clip was"
    );
    project.validate().expect("valid");
}

/// A move on the group clip moved the group as one layer; it is not something
/// the members can each carry, and the answer says it was left behind.
#[test]
fn what_the_group_clip_did_to_the_whole_is_reported_as_dropped() {
    let mut project = diagram();
    let grouped = grouping::group(&mut project, &choose(&["b1", "b2", "arrow"])).unwrap();
    let clip = project.tracks[1]
        .clips
        .iter_mut()
        .find(|c| c.id == grouped.clip)
        .expect("the group clip");
    clip.blur = 0.01;
    let back = grouping::ungroup(&mut project, &grouped.clip).unwrap();
    assert!(back.dropped_its_own_look);
}

#[test]
fn a_trimmed_group_clip_or_a_second_placement_is_refused() {
    let mut project = diagram();
    let grouped = grouping::group(&mut project, &choose(&["b1", "b2", "arrow"])).unwrap();
    let clip = project.tracks[1]
        .clips
        .iter_mut()
        .find(|c| c.id == grouped.clip)
        .expect("the group clip");
    clip.duration = Frames(10);
    let error = grouping::ungroup(&mut project, &grouped.clip).unwrap_err();
    assert!(matches!(error, UngroupError::Trimmed { .. }), "got {error}");

    let mut again = project.tracks[0].clips[0].clone();
    again.id = ClipId::new("second");
    again.asset = grouped.asset.clone();
    again.duration = Frames(10);
    project.tracks[0].clips = vec![again];
    let error = grouping::ungroup(&mut project, &grouped.clip).unwrap_err();
    assert!(
        matches!(error, UngroupError::PlacedElsewhere { .. }),
        "got {error}"
    );

    let error = grouping::ungroup(&mut project, &ClipId::new("second")).unwrap_err();
    assert!(
        error.to_string().contains("`c-group`"),
        "names the other: {error}"
    );
    let error = grouping::ungroup(&mut project, &ClipId::new("nope")).unwrap_err();
    assert!(
        matches!(error, UngroupError::NoSuchClip { .. }),
        "got {error}"
    );
}
