//! A page already in `pages/` is registered where it is, never forked into a
//! `-2` copy the project would then render instead (#1054).

use super::no_probe;
use crate::common::{new_project, source_file};
use scorsese_core::{PAGES_DIR, import_asset, write_page};

const PAGE: &str = "<!doctype html><h1>Hello</h1>";

/// Every `.html` file under `pages/`, sorted.
fn pages(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir.join(PAGES_DIR))
        .expect("pages/")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .into_string()
                .expect("utf-8")
        })
        .filter(|name| name.ends_with(".html"))
        .collect();
    names.sort();
    names
}

#[test]
fn importing_a_page_from_pages_registers_it_in_place() {
    let (dir, mut project) = new_project("page-adopt-import");
    std::fs::write(dir.join("pages/x.html"), PAGE).expect("generated");

    let id = import_asset(
        &mut project,
        &dir,
        &dir.join("pages/x.html"),
        None,
        &no_probe(),
    )
    .expect("imports");

    let asset = project.asset(&id).expect("in the table");
    assert_eq!(id.as_str(), "x");
    assert_eq!(
        asset.path.as_ref().map(|p| p.as_str()),
        Some("pages/x.html")
    );
    assert_eq!(pages(&dir), ["x.html"], "nothing copied");
}

/// The same file twice: the second import is a second document, as any
/// page imported twice is, so the first asset keeps its file.
#[test]
fn importing_a_page_another_asset_owns_still_copies() {
    let (dir, mut project) = new_project("page-adopt-owned");
    std::fs::write(dir.join("pages/x.html"), PAGE).expect("generated");
    let source = dir.join("pages/x.html");
    import_asset(&mut project, &dir, &source, None, &no_probe()).expect("first");

    let second = import_asset(&mut project, &dir, &source, None, &no_probe()).expect("second");

    let asset = project.asset(&second).expect("in the table");
    assert_eq!(
        asset.path.as_ref().map(|p| p.as_str()),
        Some("pages/x-2.html")
    );
}

#[test]
fn importing_from_outside_pages_still_copies() {
    let (dir, mut project) = new_project("page-adopt-outside");
    std::fs::write(dir.join("pages/x.html"), "<p>someone else's</p>").expect("orphan");
    let source = source_file(&dir, "x.html", PAGE.as_bytes());

    import_asset(&mut project, &dir, &source, None, &no_probe()).expect("imports");

    assert_eq!(pages(&dir), ["x-2.html", "x.html"]);
}

#[test]
fn writing_the_bytes_an_orphan_holds_adopts_it() {
    let (dir, mut project) = new_project("page-adopt-write");
    std::fs::write(dir.join("pages/x.html"), PAGE).expect("generated");

    let written = write_page(&mut project, &dir, "x", PAGE).expect("written");

    assert!(written.created);
    assert_eq!(written.path.as_str(), "pages/x.html");
    assert_eq!(pages(&dir), ["x.html"], "nothing copied");
}
