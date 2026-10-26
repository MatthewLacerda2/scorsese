//! The track questions every editor asks: where a track ends, whether a clip
//! has room on it, and keeping it in time order.

use crate::common::{asset_id, clip_id, track_id};
use scorsese_core::{Clip, Fps, Frames, Project, Track, TrackKind};

fn clip(id: &str, start: u64, duration: u64) -> Clip {
    Clip::new(clip_id(id), asset_id("a"), Frames(start), Frames(duration))
}

fn track(id: &str, kind: TrackKind, clips: Vec<Clip>) -> Track {
    let mut track = Track::new(track_id(id), kind);
    track.clips = clips;
    track
}

#[test]
fn a_track_ends_after_its_latest_clip_whatever_the_listed_order() {
    let lane = track(
        "v1",
        TrackKind::Video,
        vec![clip("late", 30, 10), clip("early", 0, 5)],
    );
    assert_eq!(lane.end(), Frames(40));
    assert_eq!(
        Track::new(track_id("e"), TrackKind::Video).end(),
        Frames::ZERO
    );
}

#[test]
fn the_timeline_runs_to_its_longest_track() {
    let mut project = Project::new("lanes", Fps::new(30, 1).expect("30fps"));
    assert_eq!(project.length(), Frames::ZERO);
    project.tracks = vec![
        track("v1", TrackKind::Video, vec![clip("a", 0, 30)]),
        track("a1", TrackKind::Audio, vec![clip("b", 10, 50)]),
    ];
    assert_eq!(project.length(), Frames(60));
}

#[test]
fn a_clip_touching_its_neighbour_fits_and_one_frame_over_does_not() {
    let lane = track(
        "v1",
        TrackKind::Video,
        vec![clip("a", 0, 30), clip("b", 60, 30)],
    );
    assert!(lane.is_free_for(&clip("new", 30, 30)));
    let over = clip("new", 29, 2);
    assert!(!lane.is_free_for(&over));
    assert_eq!(lane.first_overlap(&over).map(|c| c.id.as_str()), Some("a"));
}

#[test]
fn an_inserted_clip_lands_in_time_order() {
    let mut lane = track(
        "v1",
        TrackKind::Video,
        vec![clip("a", 0, 10), clip("c", 40, 10)],
    );
    lane.insert_clip(clip("b", 20, 10));
    let order: Vec<_> = lane.in_order().iter().map(|c| c.id.as_str()).collect();
    assert_eq!(order, ["a", "b", "c"]);
    assert_eq!(
        lane.clips[1].id.as_str(),
        "b",
        "the document itself is sorted"
    );
}

/// Room is looked for from the home track up, on tracks of the same kind.
#[test]
fn room_is_the_first_free_lane_of_the_kind_at_or_above_home() {
    let mut project = Project::new("lanes", Fps::new(30, 1).expect("30fps"));
    project.tracks = vec![
        track("v1", TrackKind::Video, vec![clip("a", 0, 30)]),
        track("a1", TrackKind::Audio, vec![]),
        track("v2", TrackKind::Video, vec![clip("b", 0, 10)]),
        track("v3", TrackKind::Video, vec![]),
    ];
    let new = clip("new", 5, 10);
    assert_eq!(project.room_for(TrackKind::Video, 0, &new), Some(3));
    assert_eq!(project.room_for(TrackKind::Audio, 0, &new), Some(1));
    assert_eq!(project.room_for(TrackKind::Audio, 2, &new), None);
    assert_eq!(
        project.room_for(TrackKind::Video, 0, &clip("n", 40, 5)),
        Some(0)
    );
}
