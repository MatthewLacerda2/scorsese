//! The files beside the pages: written and read by name, never an asset, and
//! only the kinds a page loads (#954).

use crate::common::new_project;
use scorsese_core::{MAX_PAGE_FILE_BYTES, PageFileError, Project, read_page_file, write_page_file};

const LIB: &str = "function rise(t) { return t * t; }";

#[test]
fn a_file_is_written_beside_the_pages_and_read_back() {
    let (dir, project) = new_project("page-file-new");

    let written = write_page_file(&dir, "lib.js", LIB).expect("written");

    assert!(written.created);
    assert_eq!(written.path.as_str(), "pages/lib.js");
    assert_eq!(read_page_file(&dir, "lib.js").expect("read"), LIB);
    assert_eq!(
        Project::load(&dir).expect("loads"),
        project,
        "a file beside the pages is not an asset"
    );
}

#[test]
fn the_same_name_again_replaces_it_whole() {
    let (dir, _) = new_project("page-file-again");
    write_page_file(&dir, "look.css", "h1 { color: red }").expect("first");

    let written = write_page_file(&dir, "look.css", "p { margin: 0 }").expect("second");

    assert!(!written.created);
    let on_disk = std::fs::read_to_string(dir.join("pages/look.css")).expect("on disk");
    assert_eq!(on_disk, "p { margin: 0 }");
}

#[test]
fn every_supporting_kind_is_taken_whatever_its_case() {
    let (dir, _) = new_project("page-file-kinds");
    for name in ["a.js", "b.css", "c.json", "d.svg", "E.SVG"] {
        write_page_file(&dir, name, "{}").unwrap_or_else(|error| panic!("{name}: {error}"));
    }
}

#[test]
fn a_page_or_another_kind_is_refused_and_nothing_lands() {
    let (dir, _) = new_project("page-file-kind");
    let refused = |name: &str| write_page_file(&dir, name, LIB).expect_err(name);

    assert!(matches!(refused("lib.html"), PageFileError::IsAPage { .. }));
    assert!(matches!(refused("lib.HTM"), PageFileError::IsAPage { .. }));
    for name in ["lib.py", "lib", "font.woff2"] {
        assert!(
            matches!(refused(name), PageFileError::NotAKind { .. }),
            "{name}"
        );
    }
    assert_eq!(
        std::fs::read_dir(dir.join("pages")).expect("dir").count(),
        0
    );
}

#[test]
fn a_name_that_leaves_pages_is_refused() {
    let (dir, _) = new_project("page-file-escape");
    for name in [
        "../project.json",
        "../lib.js",
        "kit/lib.js",
        ".lib.js",
        "a\\b.js",
        "",
    ] {
        let error = write_page_file(&dir, name, LIB).expect_err(name);
        assert!(
            matches!(error, PageFileError::NotAName { .. }),
            "{name}: {error}"
        );
        assert!(read_page_file(&dir, name).is_err(), "{name}");
    }
    assert!(!dir.join("lib.js").exists());
}

#[test]
fn an_empty_or_oversized_file_is_refused() {
    let (dir, _) = new_project("page-file-size");

    let empty = write_page_file(&dir, "lib.js", "  \n").expect_err("empty");
    assert!(matches!(empty, PageFileError::Empty { .. }));

    let at_limit = "x".repeat(MAX_PAGE_FILE_BYTES);
    write_page_file(&dir, "big.json", &at_limit).expect("the limit itself is kept");
    let over = format!("{at_limit}x");
    let error = write_page_file(&dir, "big.json", &over).expect_err("over");
    assert!(matches!(error, PageFileError::TooLarge { bytes, .. } if bytes == over.len()));
    let kept = std::fs::read_to_string(dir.join("pages/big.json")).expect("kept");
    assert_eq!(
        kept.len(),
        MAX_PAGE_FILE_BYTES,
        "a refusal leaves the file as it was"
    );
}

#[test]
fn reading_a_file_nobody_wrote_says_so() {
    let (dir, _) = new_project("page-file-missing");
    let error = read_page_file(&dir, "lib.js").expect_err("missing");
    assert!(matches!(error, PageFileError::Missing { .. }));
    assert_eq!(error.to_string(), "there is no pages/lib.js");
}
