//! A folder made below a base the caller trusts is never made through a link
//! on the way (#1698): what a cache writer asks before it writes.

#![cfg(unix)]

use std::os::unix::fs::symlink;
use std::path::Path;

use scorsese_core::write::{folder_below, in_cache};

use crate::common;

#[test]
fn the_folders_on_the_way_are_made() {
    let base = common::temp_project_dir("below-made");
    let made = folder_below(&base, Path::new("cache/pages/slot")).expect("made");
    assert_eq!(made, base.join("cache/pages/slot"));
    assert!(made.is_dir());
    // And again, over what is now there.
    assert_eq!(
        folder_below(&base, Path::new("cache/pages/slot")).unwrap(),
        made
    );
}

#[test]
fn a_link_on_the_way_is_refused_and_nothing_lands_where_it_leads() {
    let base = common::temp_project_dir("below-linked");
    let outside = common::temp_project_dir("below-linked-outside");
    std::fs::create_dir(base.join("slot")).unwrap();
    symlink(&outside, base.join("slot/shelf")).unwrap();
    let refused = folder_below(&base, Path::new("slot/shelf/deeper")).expect_err("a link");
    assert!(
        refused.to_string().contains("`slot/shelf` is a link"),
        "{refused}"
    );
    assert!(!outside.join("deeper").exists(), "nothing was made there");
}

#[test]
fn a_link_at_the_folder_itself_is_refused() {
    let base = common::temp_project_dir("below-last");
    let outside = common::temp_project_dir("below-last-outside");
    symlink(&outside, base.join("previews")).unwrap();
    let refused = folder_below(&base, Path::new("previews")).expect_err("a link");
    assert!(
        refused.to_string().contains("`previews` is a link"),
        "{refused}"
    );
}

#[test]
fn the_base_may_be_a_link_itself() {
    let real = common::temp_project_dir("below-base-real");
    let holder = common::temp_project_dir("below-base-holder");
    let base = holder.join("pages");
    symlink(&real, &base).unwrap();
    folder_below(&base, Path::new("slot")).expect("the base is the caller's");
    assert!(real.join("slot").is_dir());
}

#[test]
fn a_path_out_of_the_base_through_a_file_or_from_no_base_is_refused() {
    let base = common::temp_project_dir("below-refused");
    assert!(folder_below(&base, Path::new("../out")).is_err());
    assert!(folder_below(&base, Path::new("/etc")).is_err());
    std::fs::write(base.join("plain"), "a file").unwrap();
    let refused = folder_below(&base, Path::new("plain/inner")).expect_err("a file");
    assert!(
        refused.to_string().contains("is a file, not a folder"),
        "{refused}"
    );
    assert!(folder_below(&base.join("missing"), Path::new("x")).is_err());
}

/// A project's cache folder is made from its root: `cache/` itself is refused
/// when it is a link (#1718), as any folder below it is.
#[test]
fn a_cache_folder_is_made_from_the_project_root() {
    let root = common::temp_project_dir("in-cache");
    let outside = common::temp_project_dir("in-cache-outside");
    let made = in_cache(&root, Path::new("voices")).expect("made");
    assert_eq!(made, root.join("cache/voices"));
    assert!(made.is_dir());
    std::fs::remove_dir_all(root.join("cache")).unwrap();
    symlink(&outside, root.join("cache")).unwrap();

    let refused = in_cache(&root, Path::new("voices")).expect_err("a link");

    assert!(
        refused.to_string().contains("`cache` is a link"),
        "{refused}"
    );
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
}
