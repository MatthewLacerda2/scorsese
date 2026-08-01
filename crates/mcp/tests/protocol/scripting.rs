//! The script, and the reasoning a project carries about itself.
//!
//! Notes need no tools of their own — they are fields of `project.json`, so
//! `project_read` and `project_write` already carry them. The script is the
//! part that is a file beside the document, and these are the two tools that
//! exist because of that.

use super::fixture::project;
use crate::{call, said};
use serde_json::json;

/// A project with no script yet: the write has to create the file *and* point
/// the document at it, or an assistant starting one would have to hand-edit
/// `project.json` to make it findable.
#[test]
fn writing_a_script_into_a_project_that_had_none_starts_one() {
    let dir = project("script-start");
    let (text, failed) = said(&call(
        "script_write",
        json!({ "project": dir, "text": "# Brief\n\nNever call them the fleet's own cameras.\n" }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("script.md"), "got {text}");

    let document = std::fs::read_to_string(dir.join("project.json")).expect("read the document");
    assert!(
        document.contains("\"script\": \"script.md\""),
        "the document should now point at it: {document}"
    );

    let (read_back, failed) = said(&call("script_read", json!({ "project": dir })));
    assert!(!failed, "{read_back}");
    assert!(read_back.contains("Never call them"), "got {read_back}");
    std::fs::remove_dir_all(dir).ok();
}

/// Where the project already keeps its script wins over a `path` argument.
/// Moving it is a different operation, and doing it as a side effect of a write
/// would leave the old file behind with nothing pointing at it.
#[test]
fn a_second_write_replaces_the_script_where_it_already_lives() {
    let dir = project("script-replace");
    said(&call(
        "script_write",
        json!({ "project": dir, "text": "first", "path": "notes/brief.md" }),
    ));
    let (text, failed) = said(&call(
        "script_write",
        json!({ "project": dir, "text": "second", "path": "somewhere/else.md" }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("notes/brief.md"), "got {text}");
    assert!(!dir.join("somewhere/else.md").exists(), "nothing moved");

    let (read_back, _) = said(&call("script_read", json!({ "project": dir })));
    assert_eq!(read_back, "second");
    std::fs::remove_dir_all(dir).ok();
}

/// The same rule every path argument holds: it is never a way out of the
/// project directory.
#[test]
fn a_script_path_that_escapes_the_project_is_refused() {
    let dir = project("script-escape");
    let (text, failed) = said(&call(
        "script_write",
        json!({ "project": dir, "text": "no", "path": "../elsewhere.md" }),
    ));
    assert!(failed, "should have been refused: {text}");
    assert!(text.contains("escapes the project root"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// Reading a script a project does not have is an answer, not a file error —
/// and it says what to do about it.
#[test]
fn reading_a_script_from_a_project_without_one_says_how_to_start_one() {
    let dir = project("script-none");
    let (text, failed) = said(&call("script_read", json!({ "project": dir })));
    assert!(failed, "{text}");
    assert!(text.contains("script_write"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// One call says whether there is anything to read, and why the cut is the way
/// it is — ahead of the cut itself, not as a footnote under it.
#[test]
fn describing_a_project_surfaces_its_script_and_its_notes() {
    let dir = project("script-describe");
    let document = std::fs::read_to_string(dir.join("project.json")).expect("read the document");
    let annotated = document.replace(
        r#""id": "title", "kind": "text""#,
        r#""id": "title", "note": "the landing page's own headline, quoted", "kind": "text""#,
    );
    std::fs::write(dir.join("project.json"), annotated).expect("write the document");
    said(&call(
        "script_write",
        json!({ "project": dir, "text": "the brief" }),
    ));

    let (text, failed) = said(&call("project_describe", json!({ "project": dir })));
    assert!(!failed, "{text}");
    assert!(text.contains("script: script.md"), "got {text}");
    assert!(
        text.contains("note on asset `title`: the landing page's own headline, quoted"),
        "got {text}"
    );
    let (script_line, cut_line) = (
        text.find("script: script.md").expect("the script line"),
        text.find("Teaser —").expect("the cut's own line"),
    );
    assert!(script_line < cut_line, "the script comes first: {text}");
    std::fs::remove_dir_all(dir).ok();
}
