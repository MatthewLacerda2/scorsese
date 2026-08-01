//! `project_new`: the call that has to work before any of the others can be
//! made at all.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::json;

use crate::{call, said};

/// A path where no project is, and where none will be left behind.
fn somewhere(label: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "scorsese-new-{label}-{}-{unique}.scor",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// The whole point of the tool: one argument, and a project exists.
#[test]
fn one_argument_lays_out_the_directory_the_format_documents() {
    let dir = somewhere("shape");
    let (text, failed) = said(&call("project_new", json!({ "project": dir })));
    assert!(!failed, "{text}");

    for entry in ["project.json", "assets", "generated", "recipes", "cache"] {
        assert!(dir.join(entry).exists(), "a new project has no `{entry}`");
    }
    // The reply says what it made, the way the command line's does.
    assert!(text.contains(&dir.display().to_string()), "got {text}");
    assert!(text.contains("30 fps"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// The directory names the project, so the common call names nothing else.
#[test]
fn the_name_defaults_to_the_directorys_own() {
    let dir = somewhere("named-after-me");
    let (text, failed) = said(&call("project_new", json!({ "project": dir })));
    assert!(!failed, "{text}");

    let stem = dir
        .file_stem()
        .and_then(|stem| stem.to_str())
        .expect("the directory has a name")
        .to_owned();
    let document = std::fs::read_to_string(dir.join("project.json")).expect("read the document");
    assert!(
        document.contains(&format!("\"name\": \"{stem}\"")),
        "got {document}"
    );
    assert!(text.contains(&stem), "the reply says the name: {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// A name and a grid that is not the default, with the framerate as the exact
/// rational it is — 29.97 is not a framerate and is refused below.
#[test]
fn a_name_and_a_grid_are_taken_as_given() {
    let dir = somewhere("ntsc");
    let (text, failed) = said(&call(
        "project_new",
        json!({ "project": dir, "name": "A Boat At Sea", "fps": "30000/1001" }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("A Boat At Sea"), "got {text}");
    assert!(text.contains("30000/1001 fps"), "got {text}");

    let document = std::fs::read_to_string(dir.join("project.json")).expect("read the document");
    assert!(document.contains("\"num\": 30000"), "got {document}");
    std::fs::remove_dir_all(dir).ok();
}

/// A client is as likely to send `30` as `"30"`, and both are the same grid.
#[test]
fn a_framerate_is_read_as_a_number_or_as_text() {
    let dir = somewhere("plain-number");
    let (text, failed) = said(&call("project_new", json!({ "project": dir, "fps": 24 })));
    assert!(!failed, "{text}");
    assert!(text.contains("24 fps"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// A decimal is refused here for the reason it is refused on the command line:
/// 29.97 is not a framerate, and rounding it is the bug this type prevents.
#[test]
fn a_decimal_framerate_is_refused_and_nothing_is_made() {
    let dir = somewhere("decimal");
    let (text, failed) = said(&call(
        "project_new",
        json!({ "project": dir, "fps": 29.97 }),
    ));
    assert!(failed, "should have been refused: {text}");
    assert!(text.contains("30000/1001"), "got {text}");
    assert!(!dir.exists(), "a refusal made a directory anyway");
}

/// The failure that matters: an assistant re-running its own setup step must
/// not be able to blank the edit it made yesterday.
#[test]
fn a_second_call_over_a_project_refuses_and_leaves_it_alone() {
    let dir = somewhere("twice");
    said(&call(
        "project_new",
        json!({ "project": dir, "name": "the original" }),
    ));
    let (text, failed) = said(&call(
        "project_new",
        json!({ "project": dir, "name": "the replacement" }),
    ));
    assert!(failed, "a second call must not succeed: {text}");
    assert!(text.contains("already a scorsese project"), "got {text}");

    let document = std::fs::read_to_string(dir.join("project.json")).expect("read the document");
    assert!(document.contains("the original"), "it was overwritten");
    std::fs::remove_dir_all(dir).ok();
}

/// Someone made the directory first, or cloned an empty one. There is nothing
/// in it to protect, so there is nothing to refuse.
#[test]
fn an_empty_directory_that_already_exists_is_filled_in() {
    let dir = somewhere("existing-empty");
    std::fs::create_dir_all(&dir).expect("make the directory");

    let (text, failed) = said(&call("project_new", json!({ "project": dir })));
    assert!(!failed, "{text}");
    assert!(dir.join("project.json").exists(), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// Someone else's directory is not a project directory, and a project laid
/// half over one is worse than an error.
#[test]
fn a_directory_that_already_holds_something_is_refused_untouched() {
    let dir = somewhere("occupied");
    std::fs::create_dir_all(&dir).expect("make the directory");
    std::fs::write(dir.join("holiday.mp4"), b"not a project").expect("put something in it");

    let (text, failed) = said(&call("project_new", json!({ "project": dir })));
    assert!(failed, "should have been refused: {text}");
    assert!(text.contains("not empty"), "got {text}");
    assert!(!dir.join("project.json").exists(), "it wrote anyway");
    assert!(!dir.join("assets").exists(), "it made folders anyway");
    std::fs::remove_dir_all(dir).ok();
}
