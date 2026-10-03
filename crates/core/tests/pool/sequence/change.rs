//! Making a sequence from stills already in the pool, and retiming one.

use super::folder;
use crate::common::new_project;
use crate::common::stub_probe::StubProbe;
use scorsese_core::{
    AssetId, Frames, SequenceChange, SequenceError, change_sequence, import_sequence,
};

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
