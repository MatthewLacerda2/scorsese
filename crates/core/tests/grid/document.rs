//! What the document refuses now that times are frames — the whole point of
//! the schema bump is that a time nobody meant fails to load rather than
//! quietly rounding into place.

use crate::common::{asset_id, clip_id, document, project};
use scorsese_core::{Clip, Fps, Frames, LoadError, Project, SCHEMA_VERSION, TimelineProblem as E};

/// A document whose only track holds `clip`.
fn track_with(clip: &str) -> String {
    document(&format!(
        r#""tracks": [{{ "id": "v1", "kind": "video", "clips": [{clip}] }}]"#
    ))
}

#[test]
fn a_fractional_clip_time_is_refused() {
    let json = track_with(r#"{ "id": "c", "asset": "a", "start": 4.5, "duration": 60 }"#);
    let error = Project::from_json(&json).expect_err("a fractional time must fail");
    assert!(matches!(error, LoadError::Parse(_)), "got {error:?}");
}

#[test]
fn a_negative_clip_time_is_refused() {
    let json = track_with(r#"{ "id": "c", "asset": "a", "start": -1, "duration": 60 }"#);
    let error = Project::from_json(&json).expect_err("a negative time must fail");
    assert!(matches!(error, LoadError::Parse(_)), "got {error:?}");
}

#[test]
fn a_fractional_keyframe_time_is_refused() {
    let json = track_with(
        r#"{ "id": "c", "asset": "a", "start": 0, "duration": 60, "keyframes": [
            { "property": "opacity", "keyframes": [{ "t": 0.5, "value": 1.0 }] }
        ]}"#,
    );
    let error = Project::from_json(&json).expect_err("a fractional keyframe must fail");
    assert!(matches!(error, LoadError::Parse(_)), "got {error:?}");
}

/// Guessing a grid would silently re-time every clip in the document, so a
/// project without one does not load at all.
#[test]
fn a_project_without_a_timeline_fps_is_refused() {
    let json = format!(r#"{{ "schema_version": {SCHEMA_VERSION}, "name": "gridless" }}"#);
    let error = Project::from_json(&json).expect_err("a missing framerate must fail");
    assert!(matches!(error, LoadError::Parse(_)), "got {error:?}");
}

#[test]
fn an_unusable_timeline_fps_is_refused() {
    let json = format!(
        r#"{{ "schema_version": {SCHEMA_VERSION}, "name": "still",
              "timeline_fps": {{ "num": 0, "den": 1 }} }}"#
    );
    let error = Project::from_json(&json).expect_err("0fps must fail");
    assert!(matches!(error, LoadError::Parse(_)), "got {error:?}");
}

#[test]
fn a_cut_belongs_to_exactly_one_clip() {
    // c-shot runs frames 0..240 and c-title starts at 240. Frame 240 is the
    // second clip's, with nothing left to arbitrate.
    let project = project();
    let (shot, title) = (&project.tracks[0].clips[0], &project.tracks[0].clips[1]);
    assert_eq!(shot.end(), title.start);
    assert!(!shot.overlaps(title));
    // Asked the other way round too: touching is not overlapping whichever clip
    // is doing the asking, and a comparison one frame loose at either end would
    // answer the two differently.
    assert!(!title.overlaps(shot));
    assert_eq!(project.validate(), Ok(()));
}

#[test]
fn a_clip_one_frame_earlier_overlaps() {
    let mut project = project();
    project.tracks[0].clips[1].start = Frames(239);
    let (shot, title) = (&project.tracks[0].clips[0], &project.tracks[0].clips[1]);
    // Frame 239 belongs to both, which is the whole of what overlapping means —
    // and again symmetrically.
    assert!(shot.overlaps(title) && title.overlaps(shot));
    assert_eq!(project.validate().expect_err("overlap").len(), 1);
}

#[test]
fn a_new_project_starts_empty_on_the_grid_it_was_given() {
    let project = Project::new("teaser", Fps::NTSC);
    assert_eq!(project.schema_version, SCHEMA_VERSION);
    assert_eq!(project.timeline_fps, Fps::NTSC);
    assert!(project.tracks.is_empty() && project.assets.is_empty());
    assert_eq!(project.validate(), Ok(()));
}

#[test]
fn a_clip_covering_no_frame_is_reported() {
    let mut project = project();
    project.tracks[0].clips[0].duration = Frames::ZERO;
    assert_eq!(
        project.validate().expect_err("zero duration").into_vec(),
        [E::ZeroDuration {
            clip: clip_id("c-shot")
        }
        .into()]
    );
}

#[test]
fn clip_end_saturates_rather_than_overflowing() {
    // A nonsense document is a validation problem, never a panic.
    let clip = Clip::new(
        clip_id("c"),
        asset_id("a"),
        Frames(u64::MAX),
        Frames(u64::MAX),
    );
    assert_eq!(clip.end(), Frames(u64::MAX));
}
