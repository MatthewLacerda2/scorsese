//! A link already sitting at a scratch name is never written through: the
//! scratch is made fresh, and the document still lands.

#![cfg(unix)]

use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use scorsese_core::write::{atomically, fresh_beside, fresh_dir_beside, scratch_beside};

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

    plant(&file, "part", &target);

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

/// Only a taken name is passed over: any other failure is the answer at
/// once, as it was.
#[test]
fn a_scratch_that_cannot_be_made_is_the_error_itself() {
    let dir = common::temp_project_dir("write-planted-missing");
    let refused = atomically(&dir.join("missing/project.json"), "{}\n").expect_err("no folder");
    assert_eq!(refused.kind(), std::io::ErrorKind::NotFound, "{refused}");
}

/// The next `PLANTED` scratch names `suffix` would be given beside `file`,
/// each a link to `target`: the counter's current number is read off the name
/// it hands out, and the ones after it are what a writer will try.
fn plant(file: &Path, suffix: &str, target: &Path) -> Vec<PathBuf> {
    let seen = scratch_beside(file, suffix).expect("a scratch name");
    let seen = seen
        .file_name()
        .expect("a name")
        .to_string_lossy()
        .into_owned();
    let (stem, number) = seen
        .strip_suffix(&format!(".{suffix}"))
        .and_then(|s| s.rsplit_once('-'))
        .expect("<name>.<pid>-<n>.<suffix>");
    let number: u64 = number.parse().expect("a number");
    let dir = file.parent().expect("a folder");
    (1..=PLANTED)
        .map(|k| {
            let link = dir.join(format!("{stem}-{}.{suffix}", number + k));
            symlink(target, &link).expect("a link");
            link
        })
        .collect()
}

#[test]
fn a_fresh_folder_is_never_a_link_already_there() {
    let dir = common::temp_project_dir("write-planted-folder");
    let outside = common::temp_project_dir("write-planted-folder-outside");
    let planted = plant(&dir.join("part-0-30"), "capturing", &outside);

    let made = fresh_dir_beside(&dir.join("part-0-30"), "capturing").expect("a folder");

    assert!(!planted.contains(&made), "{}", made.display());
    let meta = std::fs::symlink_metadata(&made).unwrap();
    assert!(meta.is_dir() && !meta.file_type().is_symlink());
    assert_eq!(std::fs::read_dir(&made).unwrap().count(), 0, "made empty");
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
}

#[test]
fn a_fresh_file_is_never_a_link_already_there() {
    let dir = common::temp_project_dir("write-planted-file");
    let outside = common::temp_project_dir("write-planted-file-outside");
    let target = outside.join("victim.jpg");
    std::fs::write(&target, "the victim's own bytes").unwrap();
    let preview = dir.join("image-1.jpg");
    plant(&preview, "part", &target);

    let (made, mut file) = fresh_beside(&preview, "part").expect("a file");
    std::io::Write::write_all(&mut file, b"a preview").unwrap();

    assert_eq!(std::fs::read(&made).unwrap(), b"a preview");
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        "the victim's own bytes"
    );
}
