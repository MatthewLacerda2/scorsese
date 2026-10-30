//! A project file is never written through a link: a folder on the way that
//! is one is refused, and a file that is one is replaced.

#![cfg(unix)]

use std::os::unix::fs::symlink;

use scorsese_core::ProjectPath;
use scorsese_core::write::in_project;

use crate::common;

#[test]
fn a_file_lands_with_the_folders_it_needs() {
    let dir = common::temp_project_dir("in-project-lands");
    let path = ProjectPath::new("notes/deep/brief.md");
    let written = in_project(&dir, &path, "brief").expect("written");
    assert_eq!(written, dir.join("notes/deep/brief.md"));
    assert_eq!(std::fs::read_to_string(written).unwrap(), "brief");
}

#[test]
fn a_folder_that_is_a_link_is_refused_and_nothing_lands_where_it_leads() {
    let dir = common::temp_project_dir("in-project-folder-link");
    let outside = common::temp_project_dir("in-project-folder-outside");
    std::fs::create_dir_all(dir.join("cache")).unwrap();
    symlink(&outside, dir.join("cache/stock")).unwrap();
    let path = ProjectPath::new("cache/stock/pages/planted.json");
    let refused = in_project(&dir, &path, "{}").expect_err("through a link");
    assert!(
        refused.to_string().contains("`cache/stock` is a link"),
        "{refused}"
    );
    assert!(!outside.join("pages").exists(), "nothing was made there");
}

#[test]
fn a_file_that_is_a_link_is_replaced_and_what_it_named_kept() {
    let dir = common::temp_project_dir("in-project-file-link");
    let outside = common::temp_project_dir("in-project-file-outside");
    let target = outside.join("kept.md");
    std::fs::write(&target, "kept").unwrap();
    symlink(&target, dir.join("script.md")).unwrap();
    in_project(&dir, &ProjectPath::new("script.md"), "new").expect("written");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "kept");
    let meta = dir.join("script.md").symlink_metadata().unwrap();
    assert!(!meta.file_type().is_symlink());
}

/// A folder that cannot be made says why it cannot, not what the write
/// after it would then have said.
#[test]
fn a_folder_that_cannot_be_made_is_the_error() {
    use std::os::unix::fs::PermissionsExt;
    let dir = common::temp_project_dir("in-project-unmakeable");
    let shut = dir.join("shut");
    std::fs::create_dir(&shut).unwrap();
    std::fs::set_permissions(&shut, std::fs::Permissions::from_mode(0o555)).unwrap();
    let refused = in_project(&dir, &ProjectPath::new("shut/inner/x.md"), "x");
    std::fs::set_permissions(&shut, std::fs::Permissions::from_mode(0o755)).unwrap();
    let refused = refused.expect_err("a folder that cannot be made");
    assert_eq!(
        refused.kind(),
        std::io::ErrorKind::PermissionDenied,
        "{refused}"
    );
}

#[test]
fn a_path_out_of_the_project_or_through_a_file_is_refused() {
    let dir = common::temp_project_dir("in-project-refused");
    assert!(in_project(&dir, &ProjectPath::new("../out.md"), "x").is_err());
    std::fs::write(dir.join("plain"), "a file").unwrap();
    let refused = in_project(&dir, &ProjectPath::new("plain/inner.md"), "x").expect_err("a file");
    assert!(
        refused.to_string().contains("is a file, not a folder"),
        "{refused}"
    );
}
