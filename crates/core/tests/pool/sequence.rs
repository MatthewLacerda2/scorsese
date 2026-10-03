//! A folder of frames brought in as one image sequence, and sequences made
//! and changed from stills already in the pool.

use crate::common::stub_probe::StubProbe;
use crate::common::{new_project, source_file};
use scorsese_core::{
    AssetId, AssetKind, Frames, ImportError, SequenceChange, SequenceError, SkipReason,
    change_sequence, import_sequence,
};

/// A folder named `spin` holding three frames, numbered so a plain sort of
/// names would play them wrong, and two files that are not frames.
fn folder(dir: &std::path::Path) -> std::path::PathBuf {
    for (name, bytes) in [
        ("frame_10.png", "ten"),
        ("frame_2.png", "two"),
        ("frame_1.png", "one"),
        ("notes.txt", "not a frame"),
        ("wiggle.gif", "an animation"),
    ] {
        source_file(dir, &format!("spin/{name}"), bytes.as_bytes());
    }
    dir.join("sources/spin")
}

#[test]
fn a_folder_of_frames_becomes_one_sequence_in_number_order() {
    let (dir, mut project) = new_project("sequence-import");
    let report =
        import_sequence(&mut project, &dir, &folder(&dir), &StubProbe::image()).expect("import");

    assert_eq!(report.id, AssetId::new("spin"));
    let ids: Vec<&str> = report.stills.iter().map(AssetId::as_str).collect();
    assert_eq!(ids, ["spin-frame_1", "spin-frame_2", "spin-frame_10"]);
    assert!(
        dir.join("assets/spin/frame_10.png").is_file(),
        "one folder on disk"
    );
    let skipped: Vec<SkipReason> = report.skipped.iter().map(|s| s.why).collect();
    assert_eq!(skipped, [SkipReason::NotAFrame, SkipReason::NotAFrame]);
    assert_eq!(report.gaps.len(), 1, "3 to 9 are missing");
    assert_eq!(report.gaps[0].missing, 7);

    let sequence = project.asset(&report.id).expect("added");
    assert_eq!(sequence.kind, AssetKind::ImageSequence);
    let played = &sequence.sequence.as_ref().expect("its block").stills;
    assert_eq!(played, &report.stills);
    project.validate().expect("and the project validates");
}

#[test]
fn importing_the_same_folder_again_adds_nothing() {
    let (dir, mut project) = new_project("sequence-again");
    let source = folder(&dir);
    let first = import_sequence(&mut project, &dir, &source, &StubProbe::image()).expect("one");
    let count = project.assets.len();
    let again = import_sequence(&mut project, &dir, &source, &StubProbe::image()).expect("two");
    assert!(again.existed);
    assert_eq!(again.id, first.id);
    assert_eq!(project.assets.len(), count);
}

#[test]
fn frames_of_two_formats_are_refused_with_nothing_copied() {
    let (dir, mut project) = new_project("sequence-mixed");
    let source = folder(&dir);
    source_file(&dir, "spin/frame_3.jpg", b"a jpeg");
    let error =
        import_sequence(&mut project, &dir, &source, &StubProbe::image()).expect_err("two formats");
    assert!(matches!(error, ImportError::NotASequence { .. }), "{error}");
    assert!(!dir.join("assets/spin").exists());
    assert!(project.assets.is_empty());
}

#[test]
fn a_sequence_is_retimed_and_made_from_stills_already_in_the_pool() {
    let (dir, mut project) = new_project("sequence-change");
    let report =
        import_sequence(&mut project, &dir, &folder(&dir), &StubProbe::image()).expect("import");
    let loop_at_four = SequenceChange {
        hold: Some(Frames(4)),
        looping: Some(true),
        ..SequenceChange::default()
    };
    let changed = change_sequence(&mut project, &report.id, loop_at_four).expect("retime");
    assert_eq!(changed.before.map(|s| s.hold), Some(Frames(1)));
    assert!(changed.after.looping);

    let blink = AssetId::new("blink");
    let two = SequenceChange {
        stills: Some(report.stills[..2].to_vec()),
        ..SequenceChange::default()
    };
    let made = change_sequence(&mut project, &blink, two).expect("make");
    assert!(made.before.is_none());
    assert_eq!(made.after.length(), Frames(2));
}

#[test]
fn a_change_that_does_not_validate_leaves_the_project_as_it_was() {
    let (dir, mut project) = new_project("sequence-refused");
    let report =
        import_sequence(&mut project, &dir, &folder(&dir), &StubProbe::image()).expect("import");
    let before = project.clone();
    let nothing = SequenceChange {
        hold: Some(Frames(0)),
        ..SequenceChange::default()
    };
    let error = change_sequence(&mut project, &report.id, nothing).expect_err("hold of 0");
    assert!(matches!(error, SequenceError::Invalid(_)), "{error}");
    assert_eq!(project, before);

    let still = report.stills[0].clone();
    let error = change_sequence(&mut project, &still, SequenceChange::default())
        .expect_err("not a sequence");
    assert!(
        matches!(error, SequenceError::NotASequence { .. }),
        "{error}"
    );
    let ghost = AssetId::new("ghost");
    let error = change_sequence(&mut project, &ghost, SequenceChange::default())
        .expect_err("nothing to make it from");
    assert!(matches!(error, SequenceError::NoStills { .. }), "{error}");
}
