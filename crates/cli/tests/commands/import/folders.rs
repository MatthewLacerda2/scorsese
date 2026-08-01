//! `scorsese import <DIRECTORY>` — a folder of footage, brought in at once.
//!
//! A folder is a convenience for the *call*, so what is asserted here is that
//! it never becomes an asset itself, that what it does not bring in is named,
//! and that a refusal leaves the project directory exactly as it was.

use crate::common::reload;

use super::{CLIP, Outside};

/// A second clip, distinguishable from [`CLIP`] by its bytes.
const OTHER: &str = "color=c=blue:s=64x64:d=2:r=30";

/// Two clips, a licence file, and a subfolder with a third clip in it.
fn footage(outside: &Outside) {
    outside.media("wide.mp4", CLIP);
    outside.media("close.mp4", OTHER);
    outside.media("b-roll/extra.mp4", "color=c=green:s=64x64:d=2:r=30");
    std::fs::write(outside.elsewhere.join("LICENCE.txt"), b"a font's licence")
        .expect("write the licence beside the footage");
}

#[test]
fn a_folder_brings_in_the_media_directly_inside_it_and_names_what_it_did_not() {
    let outside = Outside::new("folder");
    footage(&outside);
    let folder = outside.elsewhere.clone();

    let run = outside.import(&folder, &[]).ok();
    run.says("close — Video");
    run.says("wide — Video");
    run.says("LICENCE.txt — skipped");
    run.says("b-roll — skipped");
    run.silent_about("extra");

    let project = reload(&outside.project);
    assert_eq!(project.assets.len(), 2, "the folder itself is not an asset");
    assert_eq!(outside.pool(), ["close.mp4", "wide.mp4"]);
    outside.clean();
}

#[test]
fn a_file_whose_id_is_taken_refuses_the_folder_and_copies_nothing() {
    let outside = Outside::new("folder-collision");
    footage(&outside);
    let folder = outside.elsewhere.clone();
    // Different bytes, the same name as one of the files in the folder.
    let earlier = outside.media("earlier/wide.mp4", "color=c=white:s=64x64:d=2:r=30");
    outside.import(&earlier, &[]).ok();

    let refused = outside.import(&folder, &[]);
    assert!(refused.failed, "`wide` is already an asset");
    refused.says("already an asset");

    assert_eq!(reload(&outside.project).assets.len(), 1);
    assert_eq!(outside.pool(), ["wide.mp4"], "nothing else was copied");
    outside.clean();
}
