//! `project_reframe` (#1278): the shape changes, the reply says what moved and
//! what is left, and a shape that is no frame changes nothing.

use super::fixture::{document, project, read};
use crate::{call, said};
use serde_json::json;

#[test]
fn reframing_writes_the_new_shape_and_says_what_is_left_to_settle() {
    let dir = project("reframe-tall");
    let (text, failed) = said(&call(
        "project_reframe",
        json!({ "project": dir, "aspect": "9:16" }),
    ));
    assert!(!failed, "{text}");
    assert!(text.starts_with("Reframed from 16:9 to 9:16."), "{text}");
    assert_eq!(read(&dir)["aspect"], json!("9:16"));
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_shape_that_is_no_frame_is_refused_and_changes_nothing() {
    let dir = project("reframe-ribbon");
    let before = document(&dir);
    let (text, failed) = said(&call(
        "project_reframe",
        json!({ "project": dir, "aspect": "8:1" }),
    ));
    assert!(failed, "{text}");
    assert!(text.contains("aspect:"), "{text}");
    assert_eq!(document(&dir), before);
    std::fs::remove_dir_all(dir).ok();
}
