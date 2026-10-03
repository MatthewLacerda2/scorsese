//! Removing an asset or a lane: the first call asks what would go, and only a
//! second call naming those clips destroys anything.

use super::fixture::project;
use crate::{call, said};
use serde_json::json;

/// The document, for a test that has to see what actually landed in it.
fn document(dir: &std::path::Path) -> String {
    std::fs::read_to_string(dir.join("project.json")).expect("the project is on disk")
}

/// The confirmation round-trip #396 settled on: the refusal names the clip,
/// and the call that names it back removes the asset and the clip together.
#[test]
fn an_asset_in_use_goes_only_on_the_second_call_naming_its_clips() {
    let dir = project("asset-remove");
    let before = document(&dir);
    let (text, failed) = said(&call(
        "asset_remove",
        json!({ "project": dir, "asset": "title" }),
    ));
    assert!(failed, "`c1` shows the title");
    assert!(text.contains("`c1`"), "the refusal names the clip: {text}");
    assert!(text.contains("nothing was written"), "got {text}");
    assert_eq!(document(&dir), before);

    let (text, failed) = said(&call(
        "asset_remove",
        json!({ "project": dir, "asset": "title", "clips": ["c1"] }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("`c1` from `v1`"), "got {text}");
    let written = document(&dir);
    assert!(!written.contains("\"title\""), "the asset went: {written}");
    assert!(!written.contains("\"c1\""), "and its clip: {written}");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_lane_goes_with_its_clips_named_and_its_assets_stay() {
    let dir = project("track-remove");
    let (text, failed) = said(&call(
        "track_remove",
        json!({ "project": dir, "track": "v1", "clips": [] }),
    ));
    assert!(failed, "`c1` is on v1");
    assert!(text.contains("`c1`"), "got {text}");

    let (text, failed) = said(&call(
        "track_remove",
        json!({ "project": dir, "track": "v1", "clips": ["c1"] }),
    ));
    assert!(!failed, "{text}");
    let written = document(&dir);
    assert!(!written.contains("\"v1\""), "the lane went: {written}");
    assert!(written.contains("\"title\""), "the asset stayed: {written}");
    std::fs::remove_dir_all(dir).ok();
}
