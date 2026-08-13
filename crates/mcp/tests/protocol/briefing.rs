//! Editing a brief, and the state that has to move with it.

use super::fixture::{DOCUMENT, project};
use crate::{call, said};
use serde_json::{Value, json};

/// The fixture, with `vo` already generated — the one state the edit has
/// somewhere to move from.
fn generated(label: &str) -> std::path::PathBuf {
    let dir = project(label);
    let document = DOCUMENT.replace(
        r#""prompt": "a line", "state": "sketch""#,
        r#""prompt": "a line", "state": "generated", "path": "generated/vo.mp3""#,
    );
    std::fs::write(dir.join("project.json"), document).expect("write the document");
    dir
}

/// What `project.json` says now, parsed.
fn on_disk(dir: &std::path::Path) -> Value {
    let text = std::fs::read_to_string(dir.join("project.json")).expect("read back");
    serde_json::from_str(&text).expect("the document is JSON")
}

/// The asset row for `id`.
fn asset(document: &Value, id: &str) -> Value {
    document["assets"]
        .as_array()
        .expect("assets")
        .iter()
        .find(|row| row["id"] == json!(id))
        .unwrap_or_else(|| panic!("no asset {id}"))
        .clone()
}

/// The whole point: one call writes the new brief *and* the state that says the
/// file on disk is no longer what the project asks for.
#[test]
fn editing_a_generated_prompt_marks_it_stale_in_the_same_write() {
    let dir = generated("rebrief-stale");
    let (text, failed) = said(&call(
        "rebrief",
        json!({ "project": dir, "asset": "vo", "prompt": "a better line" }),
    ));
    assert!(!failed, "{text}");
    assert!(
        text.contains("stale"),
        "the reply must state the state: {text}"
    );

    let vo = asset(&on_disk(&dir), "vo");
    assert_eq!(vo["prompt"], json!("a better line"));
    assert_eq!(vo["state"], json!("stale"));
    std::fs::remove_dir_all(dir).ok();
}

/// A prompt and a recipe are two kinds of brief and an asset carries exactly
/// one. Handing over the other is a misunderstanding of the asset, so it is
/// refused rather than resolved.
#[test]
fn the_brief_a_kind_does_not_take_is_refused_and_changes_nothing() {
    let dir = generated("rebrief-mismatch");
    let before = std::fs::read_to_string(dir.join("project.json")).expect("read");

    let (text, failed) = said(&call(
        "rebrief",
        json!({ "project": dir, "asset": "vo", "recipe": "recipes/bed.json" }),
    ));
    assert!(
        failed,
        "a recipe on a prompted asset must be refused: {text}"
    );
    let (also, refused) = said(&call(
        "rebrief",
        json!({ "project": dir, "asset": "bed", "prompt": "something jaunty" }),
    ));
    assert!(refused, "a prompt on a synth asset must be refused: {also}");

    assert_eq!(
        std::fs::read_to_string(dir.join("project.json")).expect("read"),
        before
    );
    std::fs::remove_dir_all(dir).ok();
}

/// The synthesised half: the brief is the recipe the asset points at, and
/// repointing it is the same edit. `bed` has never been baked, so the honest
/// state is still `sketch` — and the reply says so rather than claiming a
/// generation that never happened.
#[test]
fn repointing_a_synth_asset_at_another_recipe_leaves_a_sketch_a_sketch() {
    let dir = project("rebrief-recipe");
    std::fs::copy(dir.join("recipes/bed.json"), dir.join("recipes/other.json")).expect("copy");

    let (text, failed) = said(&call(
        "rebrief",
        json!({ "project": dir, "asset": "bed", "recipe": "recipes/other.json" }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("sketch"), "got {text}");
    assert_eq!(
        asset(&on_disk(&dir), "bed")["recipe"],
        json!("recipes/other.json")
    );
    std::fs::remove_dir_all(dir).ok();
}

/// A recipe that is not there is a typo, and pointing an asset at it would make
/// a project that bakes to nothing.
#[test]
fn a_recipe_that_is_not_there_is_refused() {
    let dir = project("rebrief-missing");
    let (text, failed) = said(&call(
        "rebrief",
        json!({ "project": dir, "asset": "bed", "recipe": "recipes/ghost.json" }),
    ));
    assert!(failed && text.contains("ghost"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// A queued asset has been paid for and the result is named after the brief
/// that was sent. Editing now would file what arrives under a name it is not.
#[test]
fn a_brief_in_flight_is_refused_until_it_is_collected() {
    let dir = project("rebrief-queued");
    let document = DOCUMENT.replace(
        r#""prompt": "a line", "state": "sketch""#,
        r#""prompt": "a line", "state": "queued""#,
    );
    std::fs::write(dir.join("project.json"), document).expect("write");

    let (text, failed) = said(&call(
        "rebrief",
        json!({ "project": dir, "asset": "vo", "prompt": "a better line" }),
    ));
    assert!(failed && text.contains("collect"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// Writing the brief that is already there would cost a slug card in every
/// preview until somebody regenerated an asset nothing had changed.
#[test]
fn a_brief_that_already_reads_that_way_is_left_generated() {
    let dir = generated("rebrief-same");
    let (text, failed) = said(&call(
        "rebrief",
        json!({ "project": dir, "asset": "vo", "prompt": "a line" }),
    ));
    assert!(!failed, "{text}");
    assert_eq!(asset(&on_disk(&dir), "vo")["state"], json!("generated"));
    std::fs::remove_dir_all(dir).ok();
}

/// An asset with no brief has nothing for this tool to edit.
#[test]
fn an_asset_that_is_not_generated_has_no_brief() {
    let dir = project("rebrief-text");
    let (text, failed) = said(&call(
        "rebrief",
        json!({ "project": dir, "asset": "title", "prompt": "TRAILER" }),
    ));
    assert!(failed && text.contains("text"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}
