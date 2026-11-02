//! What a clone is sent from is bounded (#1649): a recording named twice is
//! sent once, and more recordings or more speech than a clone takes is
//! refused before a file is read.

use scorsese_core::{AssetId, ProjectPath};
use scorsese_providers::voices::clone::{clone, cloned};

use super::cloning::{ask, recorded};
use super::fake::Fake;

#[test]
fn a_recording_named_twice_is_read_and_sent_once() {
    let studio = Fake::offering(&[]);
    let (dir, project) = recorded("clone-twice", 90.0);

    clone(&dir, &project, &studio, &ask(&["take", "take"], true)).expect("the clone");

    let sent = studio.cloned.borrow();
    assert_eq!(sent[0].recordings.len(), 1);
    assert_eq!(cloned(&dir)[0].recordings.len(), 1);
}

#[test]
fn two_assets_of_one_file_are_one_recording() {
    let studio = Fake::offering(&[]);
    let (dir, mut project) = recorded("clone-one-file", 90.0);
    let mut again = project.assets[0].clone();
    again.id = AssetId::new("again");
    project.assets.push(again);

    clone(&dir, &project, &studio, &ask(&["take", "again"], true)).expect("the clone");

    assert_eq!(studio.cloned.borrow()[0].recordings.len(), 1);
}

#[test]
fn more_recordings_or_more_speech_than_a_clone_takes_is_refused_before_reading() {
    let studio = Fake::offering(&[]);
    let (dir, mut project) = recorded("clone-too-many", 90.0);
    let take = project.assets[0].clone();
    let mut named = Vec::new();
    for n in 0..26 {
        let mut copy = take.clone();
        copy.id = AssetId::new(format!("take{n}"));
        copy.path = Some(ProjectPath::new(format!("assets/missing{n}.mp3")));
        copy.sha256 = Some(format!("{n:064}"));
        project.assets.push(copy);
        named.push(format!("take{n}"));
    }
    let named: Vec<&str> = named.iter().map(String::as_str).collect();

    // Not one of the files exists: refused before any is read.
    let error = clone(&dir, &project, &studio, &ask(&named, true)).expect_err("26 recordings");
    assert!(error.to_string().contains("25"), "{error}");
    let error = clone(&dir, &project, &studio, &ask(&named[..7], true)).expect_err("10.5 minutes");
    assert!(error.to_string().contains("minutes"), "{error}");
    assert!(studio.cloned.borrow().is_empty());
}
