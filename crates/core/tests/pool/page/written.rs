//! Writing a page in place: one call makes it, the same call edits it.

use crate::common::new_project;
use scorsese_core::{
    Asset, AssetId, AssetKind, PAGES_DIR, PageError, Project, ProjectPath, write_page,
};

const PAGE: &str = "<!doctype html><h1>Hello</h1>";

#[test]
fn a_new_name_makes_the_asset_and_its_file() {
    let (dir, mut project) = new_project("page-write-new");

    let written = write_page(&mut project, &dir, "title", PAGE).expect("written");

    assert!(written.created);
    assert_eq!(written.id.as_str(), "title");
    assert_eq!(written.path.as_str(), "pages/title.html");
    let asset = project.asset(&written.id).expect("in the table");
    assert_eq!(asset.kind, AssetKind::Html);
    assert_eq!(asset.sha256, None, "authored, so never hashed");
    let on_disk = std::fs::read_to_string(dir.join("pages/title.html")).expect("on disk");
    assert_eq!(on_disk, PAGE);

    project.save(&dir).expect("save");
    Project::load(&dir).expect("a written page loads and validates");
}

#[test]
fn the_same_name_again_rewrites_the_file_and_nothing_else() {
    let (dir, mut project) = new_project("page-write-again");
    write_page(&mut project, &dir, "title", PAGE).expect("first");
    let before = project.clone();

    let written = write_page(&mut project, &dir, "title", "<p>Bye</p>").expect("second");

    assert!(!written.created);
    assert_eq!(written.path.as_str(), "pages/title.html");
    assert_eq!(
        project, before,
        "a rewrite changes no field of the document"
    );
    let on_disk = std::fs::read_to_string(dir.join("pages/title.html")).expect("on disk");
    assert_eq!(on_disk, "<p>Bye</p>");
}

/// A file already in `pages/` that no asset names is somebody's work: a new
/// page goes beside it rather than over it.
#[test]
fn a_file_nobody_names_is_left_alone() {
    let (dir, mut project) = new_project("page-write-beside");
    let orphan = dir.join(PAGES_DIR).join("title.html");
    std::fs::write(&orphan, "kept").expect("orphan");

    let written = write_page(&mut project, &dir, "title", PAGE).expect("written");

    assert_eq!(written.path.as_str(), "pages/title-2.html");
    assert_eq!(
        std::fs::read_to_string(orphan).expect("still there"),
        "kept"
    );
}

#[test]
fn a_name_of_another_kind_is_refused_with_nothing_written() {
    let (dir, mut project) = new_project("page-write-taken");
    project.assets.push(Asset::imported(
        AssetId::new("title"),
        AssetKind::Image,
        ProjectPath::new("assets/title.png"),
    ));
    let before = project.clone();

    let error = write_page(&mut project, &dir, "title", PAGE).expect_err("refused");

    assert!(matches!(error, PageError::NotAPage { .. }), "{error}");
    assert!(format!("{error}").contains("an `image` asset"), "{error}");
    assert_eq!(project, before);
    assert_eq!(
        std::fs::read_dir(dir.join(PAGES_DIR)).expect("dir").count(),
        0
    );
}

#[test]
fn an_empty_document_is_refused() {
    let (dir, mut project) = new_project("page-write-empty");
    let error = write_page(&mut project, &dir, "title", "  \n").expect_err("refused");
    assert!(matches!(error, PageError::Empty), "{error}");
    assert!(project.assets.is_empty());
}
