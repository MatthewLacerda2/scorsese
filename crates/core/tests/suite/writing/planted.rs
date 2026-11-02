//! A link already sitting at a scratch name is never written through: the
//! scratch is made fresh, and the document still lands.

#![cfg(unix)]

use std::os::unix::fs::symlink;
use std::path::PathBuf;

use scorsese_core::write::{atomically, scratch_beside};

use crate::common;

/// How many of the next scratch names get a link: more than one, so a call
/// taken by another test on this process's counter does not slip past them.
const PLANTED: u64 = 16;

#[test]
fn a_link_at_the_scratch_name_is_neither_written_nor_truncated() {
    let dir = common::temp_project_dir("write-planted");
    let outside = common::temp_project_dir("write-planted-outside");
    let target = outside.join("victim.txt");
    std::fs::write(&target, "the victim's own bytes").unwrap();
    let file = dir.join("project.json");

    // The counter's current number, read off the name it hands out; the next
    // ones are what `atomically` will try.
    let seen = scratch_beside(&file, "part").unwrap();
    let seen = seen.file_name().unwrap().to_string_lossy().into_owned();
    let (stem, number) = seen
        .strip_suffix(".part")
        .and_then(|s| s.rsplit_once('-'))
        .expect("<name>.<pid>-<n>.part");
    let number: u64 = number.parse().unwrap();
    let planted: Vec<PathBuf> = (1..=PLANTED)
        .map(|k| dir.join(format!("{stem}-{}.part", number + k)))
        .collect();
    for link in &planted {
        symlink(&target, link).unwrap();
    }

    atomically(&file, "{}\n").expect("the document still lands");

    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        "the victim's own bytes"
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "{}\n");
    assert!(
        !std::fs::symlink_metadata(&file)
            .unwrap()
            .file_type()
            .is_symlink(),
        "the document is a file of its own, not the link"
    );
}
