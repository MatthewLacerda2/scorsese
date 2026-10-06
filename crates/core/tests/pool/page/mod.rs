//! Bringing a page in: copied into `pages/`, neither probed nor hashed.

mod kept;
mod written;

use crate::common::stub_probe::StubProbe;
use crate::common::{new_project, source_file};
use scorsese_core::{
    AssetKind, ImportError, PAGES_DIR, Project, SkipReason, import_asset, import_path,
};

const PAGE: &[u8] = b"<!doctype html><h1>Hello</h1>";

/// A probe that fails every call: a page reaching it is the bug.
fn no_probe() -> StubProbe {
    StubProbe::failing("a page must never be probed")
}

#[test]
fn a_page_is_copied_into_pages_and_neither_probed_nor_hashed() {
    let (dir, mut project) = new_project("page-import");
    let source = source_file(&dir, "Title.html", PAGE);

    let id = import_asset(&mut project, &dir, &source, None, &no_probe()).expect("imports");

    let asset = project.asset(&id).expect("in the table");
    assert_eq!(id.as_str(), "title");
    assert_eq!(asset.kind, AssetKind::Html);
    assert_eq!(
        asset.path.as_ref().map(|p| p.as_str()),
        Some("pages/title.html")
    );
    assert_eq!(asset.sha256, None, "an authored page carries no hash");
    assert_eq!(asset.media, None, "nothing was probed");
    assert_eq!(
        std::fs::read(dir.join(PAGES_DIR).join("title.html")).expect("copied"),
        PAGE
    );
    assert!(source.is_file(), "the original is left alone");
}

#[test]
fn an_imported_page_loads_and_validates() {
    let (dir, mut project) = new_project("page-valid");
    let source = source_file(&dir, "lower-third.html", PAGE);
    import_asset(&mut project, &dir, &source, None, &no_probe()).expect("imports");

    project.save(&dir).expect("save");
    let reloaded = Project::load(&dir).expect("load validates");
    assert_eq!(reloaded.assets[0].kind, AssetKind::Html);
}

/// Two documents about to diverge, not one asset twice.
#[test]
fn the_same_page_twice_is_two_pages() {
    let (dir, mut project) = new_project("page-twice");
    let source = source_file(&dir, "title.html", PAGE);

    let first = import_asset(&mut project, &dir, &source, None, &no_probe()).expect("first");
    let second = import_asset(&mut project, &dir, &source, None, &no_probe()).expect("second");

    assert_eq!((first.as_str(), second.as_str()), ("title", "title-2"));
    assert!(dir.join("pages/title-2.html").is_file());
}

/// `--kind html` on a file that is not one: refused before anything lands.
#[test]
fn a_page_without_the_extension_is_refused_with_nothing_copied() {
    let (dir, mut project) = new_project("page-not-html");
    let source = source_file(&dir, "title.txt", PAGE);

    let error = import_asset(
        &mut project,
        &dir,
        &source,
        Some(AssetKind::Html),
        &no_probe(),
    )
    .expect_err("refused");

    assert!(matches!(error, ImportError::KindMismatch { .. }), "{error}");
    assert!(project.assets.is_empty());
    assert_eq!(
        std::fs::read_dir(dir.join(PAGES_DIR))
            .expect("exists")
            .count(),
        0
    );
}

/// Pages come in one at a time: a folder of footage passes its page over.
#[test]
fn a_directory_passes_a_page_over() {
    let (dir, mut project) = new_project("page-directory");
    source_file(&dir, "clip.mp4", b"a video");
    source_file(&dir, "index.html", PAGE);

    let report = import_path(
        &mut project,
        &dir,
        &dir.join("sources"),
        None,
        &StubProbe::video(),
    )
    .expect("imports");

    assert_eq!(report.imported.len(), 1);
    assert_eq!(report.skipped.len(), 1);
    assert_eq!(report.skipped[0].source, "index.html");
    assert_eq!(report.skipped[0].why, SkipReason::UnknownKind);
}

#[test]
fn a_new_project_has_a_pages_directory() {
    let (dir, _) = new_project("page-dir");
    assert!(dir.join(PAGES_DIR).is_dir());
}
