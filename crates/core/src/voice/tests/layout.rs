//! Where a cut puts things.

use super::{at, project, voicing};
use crate::voice::{Measured, cut_to_voice};
use crate::{ClipId, Frames, Track, TrackId, TrackKind};

#[test]
fn each_scene_ends_the_gap_after_its_lines_last_word() {
    let (dir, mut project) = project("gap");
    let voiced = cut_to_voice(&mut project, &dir, &voicing()).unwrap();

    // Line 1's last word ends 1.5 s in: + 0.2 s is 1.7 s, frame 51. Line 2
    // starts there and its last word ends 2.0 s in: 3.9 s, frame 117. Line 3
    // has no timings, so its 60 frames are what it says: 177, + 6 is 183.
    let spans: Vec<(u64, u64, Measured)> = voiced
        .scenes
        .iter()
        .map(|laid| (laid.start.get(), laid.end.get(), laid.measured))
        .collect();
    assert_eq!(
        spans,
        [
            (0, 51, Measured::LastWord),
            (51, 117, Measured::LastWord),
            (117, 183, Measured::EndOfAudio),
        ]
    );
    assert_eq!(voiced.was, Frames(260));
    assert_eq!(at(&project, "page-1"), ("v1".into(), 0, 51));
    assert_eq!(at(&project, "page-2"), ("v1".into(), 51, 117));
    assert_eq!(at(&project, "page-3"), ("v1".into(), 117, 183));
    assert_eq!(at(&project, "nar-2").1, 51);
    assert_eq!(at(&project, "nar-3").1, 117);
}

#[test]
fn a_lines_trailing_silence_is_cut_where_the_next_line_starts() {
    let (dir, mut project) = project("tails");
    cut_to_voice(&mut project, &dir, &voicing()).unwrap();
    assert_eq!(at(&project, "nar-1"), ("a1".into(), 0, 51));
    assert_eq!(at(&project, "nar-2"), ("a1".into(), 51, 117));
    assert_eq!(at(&project, "nar-3"), ("a1".into(), 117, 177), "kept whole");
}

#[test]
fn a_rider_keeps_its_offset_and_what_is_not_named_stays_put() {
    let (dir, mut project) = project("riders");
    let voiced = cut_to_voice(&mut project, &dir, &voicing()).unwrap();
    // Five frames into scene 2 before, five frames into it after.
    assert_eq!(at(&project, "whoosh"), ("sfx".into(), 56, 66));
    assert_eq!(at(&project, "bed"), ("music".into(), 0, 300));
    assert!(
        voiced.crossed.is_empty(),
        "the bed still runs the whole cut"
    );
    assert!(voiced.rearranged.is_empty());
}

#[test]
fn a_lead_in_delays_the_line_and_so_the_scenes_end() {
    let (dir, mut project) = project("lead");
    let mut voicing = voicing();
    voicing.lead_in = 0.5;
    voicing.scenes[1].lead_in = Some(0.0);
    voicing.from = Some(1.0);
    let voiced = cut_to_voice(&mut project, &dir, &voicing).unwrap();
    // Scene 1 begins at 1 s; its line 0.5 s later, so its last word ends at
    // 3.0 s and the scene at 3.2 s, frame 96. Scene 2 overrides to no lead.
    assert_eq!(at(&project, "nar-1").1, 45);
    assert_eq!(
        (voiced.scenes[0].start, voiced.scenes[0].end),
        (Frames(30), Frames(96))
    );
    assert_eq!(at(&project, "nar-2").1, 96);
}

#[test]
fn an_overlap_runs_the_outgoing_visual_on_and_moves_the_incoming_one_up() {
    let (dir, mut project) = project("overlap");
    let mut voicing = voicing();
    voicing.overlap = 0.2;
    let voiced = cut_to_voice(&mut project, &dir, &voicing).unwrap();
    assert_eq!(at(&project, "page-1"), ("v1".into(), 0, 57));
    assert_eq!(at(&project, "page-2"), ("v2".into(), 51, 123));
    // Room again on the first track: the scenes alternate.
    assert_eq!(
        at(&project, "page-3"),
        ("v1".into(), 117, 183),
        "the last has nothing to run under"
    );
    assert_eq!(
        voiced.rearranged,
        [(ClipId::new("page-2"), TrackId::new("v2"), true)]
    );
    assert!(voiced.crossed.is_empty(), "named clips are never reported");
    let order: Vec<&str> = project.tracks.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(order[..2], ["v1", "v2"], "made directly above its own");
}

#[test]
fn an_unnamed_clip_whose_scene_changed_is_reported() {
    let (dir, mut project) = project("crossed");
    let mut overlay = Track::new(TrackId::new("over"), TrackKind::Video);
    overlay.clips.push(crate::Clip::new(
        ClipId::new("badge"),
        crate::AssetId::new("still"),
        Frames(150),
        Frames(20),
    ));
    project.tracks.push(overlay);
    let voiced = cut_to_voice(&mut project, &dir, &voicing()).unwrap();
    assert_eq!(
        voiced.crossed,
        [ClipId::new("badge")],
        "scene 2 before, scene 3 now"
    );
    assert_eq!(at(&project, "badge"), ("over".into(), 150, 170));
}

#[test]
fn a_rerun_on_the_cut_it_made_changes_nothing() {
    let (dir, mut project) = project("rerun");
    cut_to_voice(&mut project, &dir, &voicing()).unwrap();
    let once = project.clone();
    cut_to_voice(&mut project, &dir, &voicing()).unwrap();
    assert_eq!(project, once);
}

/// A chunk of narration with a picture change inside it (#1111): the second
/// picture sits on a track above the scene's visual, starts on a word's time,
/// runs to the scene's end and rides the scene.
fn a_picture_changed_mid_chunk(label: &str, overlap: u64) -> crate::Project {
    let (dir, mut project) = project(label);
    // `nar-2` starts at frame 100 and says "line." 0.4 s in: frame 112. Its
    // scene ends 2.2 s after the line starts, frame 166, plus any overlap.
    let up = Track::with_clips(
        "v2",
        TrackKind::Video,
        vec![crate::Clip::at("page-2b", "still", 112, 54 + overlap)],
    );
    project.tracks.insert(1, up);
    let mut voicing = voicing();
    voicing.overlap = overlap as f64 / 30.0;
    voicing.scenes[1].riders.push(ClipId::new("page-2b"));
    let voiced = cut_to_voice(&mut project, &dir, &voicing).unwrap();
    assert!(voiced.crossed.is_empty());
    project
}

#[test]
fn a_picture_change_inside_a_chunk_rides_its_scene_to_the_word() {
    let project = a_picture_changed_mid_chunk("mid-chunk", 0);
    // Scene 2 now runs 51–117: the picture still arrives on "line.", twelve
    // frames in, and still ends with the scene.
    assert_eq!(at(&project, "page-2"), ("v1".into(), 51, 117));
    assert_eq!(at(&project, "page-2b"), ("v2".into(), 63, 117));
}

#[test]
fn with_an_overlap_the_change_inside_a_chunk_stays_above_its_scene() {
    let project = a_picture_changed_mid_chunk("mid-chunk-overlap", 6);
    // `page-2` runs on under scene 3 and so moves up to `v2`; the change
    // inside its chunk, laid after it, goes up again and stays on top.
    assert_eq!(at(&project, "page-2"), ("v2".into(), 51, 123));
    assert_eq!(at(&project, "page-2b"), ("v3".into(), 63, 123));
    let order: Vec<String> = project.tracks.iter().map(|t| t.id.to_string()).collect();
    assert_eq!(order[..3], ["v1", "v2", "v3"]);
}
