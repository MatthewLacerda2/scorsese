//! The primitive itself: what ends up on disk, and what is left beside it.

use std::path::{Path, PathBuf};

use scorsese_core::write::atomically;

use crate::common;

/// What is in a directory, by name, sorted — so a leftover scratch file shows
/// up as a difference rather than having to be looked for.
fn entries(dir: &Path) -> Vec<PathBuf> {
    let mut names: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("read the directory")
        .map(|entry| PathBuf::from(entry.expect("an entry").file_name()))
        .collect();
    names.sort();
    names
}

#[test]
fn a_document_lands_where_it_was_asked_to() {
    let dir = common::temp_project_dir("write-lands");
    let file = dir.join("project.json");

    atomically(&file, "{}\n").expect("write");

    assert_eq!(
        std::fs::read_to_string(&file).expect("read it back"),
        "{}\n"
    );
}

/// The failure a naive write-in-place has: a shorter document leaves the tail
/// of the longer one behind it. A rename cannot, because what lands is a whole
/// file rather than an edit to one.
#[test]
fn writing_a_shorter_document_leaves_none_of_the_longer_one() {
    let dir = common::temp_project_dir("write-shorter");
    let file = dir.join("project.json");

    atomically(&file, "a".repeat(4096)).expect("the long one");
    atomically(&file, "short").expect("the short one");

    assert_eq!(std::fs::read_to_string(&file).expect("read"), "short");
}

#[test]
fn a_finished_write_leaves_nothing_beside_it() {
    let dir = common::temp_project_dir("write-tidy");

    atomically(&dir.join("project.json"), "{}\n").expect("write");

    assert_eq!(entries(&dir), [PathBuf::from("project.json")]);
}

/// A write that cannot complete must cost nothing: the previous document is
/// still whole, and no debris is left in the project directory.
#[test]
fn a_failed_write_leaves_the_directory_as_it_found_it() {
    let dir = common::temp_project_dir("write-refused");
    let occupied = dir.join("occupied");
    std::fs::create_dir(&occupied).expect("a directory where the file would go");

    let refused = atomically(&occupied, "never lands");

    assert!(
        refused.is_err(),
        "renaming a file over a directory must fail"
    );
    assert_eq!(entries(&dir), [PathBuf::from("occupied")]);
}

/// Writers must not land on one another either. Eight replace one document at
/// once with payloads of different lengths and different bytes; what is there
/// at the end has to be exactly one of them, never the head of one and the
/// tail of another.
#[test]
fn concurrent_writers_never_splice_their_documents() {
    let dir = common::temp_project_dir("write-race");
    let file = dir.join("project.json");
    let payloads: Vec<String> = "abcdefgh"
        .chars()
        .enumerate()
        .map(|(n, ch)| ch.to_string().repeat(4096 * (n + 1)))
        .collect();

    let target = &file;
    std::thread::scope(|scope| {
        for payload in &payloads {
            scope.spawn(move || atomically(target, payload).expect("write"));
        }
    });

    let found = std::fs::read_to_string(&file).expect("read");
    assert!(
        payloads.contains(&found),
        "the document is a splice, not one of the writes: {} bytes",
        found.len()
    );
    assert_eq!(entries(&dir), [PathBuf::from("project.json")]);
}
