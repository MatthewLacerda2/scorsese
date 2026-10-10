//! The files beside the pages, through the page tools (#954): `file` instead
//! of `page`, never an asset, and only the kinds a page loads.

use super::fixture::project;
use crate::{call, said};
use serde_json::json;

#[test]
fn page_write_with_file_writes_beside_the_pages_and_page_read_reads_it() {
    let dir = project("page-file");
    let before = std::fs::read_to_string(dir.join("project.json")).expect("document");

    let lib = "function rise(t) { return t * t; }";
    let (text, failed) = said(&call(
        "page_write",
        json!({ "project": dir, "file": "lib.js", "html": lib }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("pages/lib.js written"), "{text}");
    let (text, failed) = said(&call(
        "page_write",
        json!({ "project": dir, "file": "lib.js", "html": "function fall() {}" }),
    ));
    assert!(!failed && text.contains("rewritten"), "{text}");

    let (read, failed) = said(&call(
        "page_read",
        json!({ "project": dir, "file": "lib.js" }),
    ));
    assert!(!failed, "{read}");
    assert_eq!(read, "function fall() {}");
    let after = std::fs::read_to_string(dir.join("project.json")).expect("document");
    assert_eq!(after, before, "a file beside the pages is not an asset");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_page_name_or_a_name_outside_pages_is_refused_as_a_file() {
    let dir = project("page-file-refused");
    for (file, says) in [
        ("card.html", "written by its asset id"),
        ("lib.py", ".js, .css, .json or .svg"),
        ("../project.json", "not a file name under pages/"),
    ] {
        let (text, failed) = said(&call(
            "page_write",
            json!({ "project": dir, "file": file, "html": "x" }),
        ));
        assert!(failed && text.contains("nothing written"), "{file}: {text}");
        assert!(text.contains(says), "{file}: {text}");
    }
    assert!(!dir.join("pages").exists(), "no file landed");

    let (text, failed) = said(&call(
        "page_read",
        json!({ "project": dir, "file": "lib.js" }),
    ));
    assert!(
        failed && text.contains("there is no pages/lib.js"),
        "{text}"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_call_names_a_page_or_a_file_and_not_both() {
    let dir = project("page-file-which");
    for tool in ["page_write", "page_read"] {
        let (text, failed) = said(&call(tool, json!({ "project": dir, "html": "<p>x</p>" })));
        assert!(
            failed && text.contains("`page` is required"),
            "{tool}: {text}"
        );

        let both = json!({ "project": dir, "page": "card", "file": "lib.js", "html": "x" });
        let (text, failed) = said(&call(tool, both));
        assert!(failed && text.contains("not both"), "{tool}: {text}");
    }
    std::fs::remove_dir_all(dir).ok();
}
