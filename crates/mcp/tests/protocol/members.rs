//! A group's members are clips like any other: every per-clip tool takes a
//! member's id, and the edits that cannot leave the timeline say so by naming
//! the group (#1103).

use super::fixture::{DOCUMENT, document, project, read};
use crate::{call, said};
use serde_json::{Value, json};

/// The fixture with a second title, `c2` on `v2`, and both titles wrapped into
/// the group `titles`.
fn grouped(label: &str) -> std::path::PathBuf {
    let dir = project(label);
    let document = DOCUMENT.replace(
        r#"    { "id": "music", "kind": "audio","#,
        r#"    { "id": "v2", "kind": "video",
      "clips": [ { "id": "c2", "asset": "title", "start": 0, "duration": 300 } ] },
    { "id": "music", "kind": "audio","#,
    );
    std::fs::write(dir.join("project.json"), document).expect("write the document");
    let (text, failed) = said(&call(
        "clip_group",
        json!({ "project": dir, "clips": ["c1", "c2"], "asset": "titles" }),
    ));
    assert!(!failed, "{text}");
    dir
}

/// The member called `id`, as the document on disk has it.
fn member(dir: &std::path::Path, id: &str) -> Option<Value> {
    let document = read(dir);
    let group = document["assets"]
        .as_array()?
        .iter()
        .find(|asset| asset["id"] == "titles")?
        .clone();
    group["group"]["tracks"]
        .as_array()?
        .iter()
        .flat_map(|track| track["clips"].as_array().cloned().unwrap_or_default())
        .find(|clip| clip["id"] == id)
}

#[test]
fn every_per_clip_tool_reaches_a_member() {
    let dir = grouped("members");
    let edits = [
        (
            "clip_set",
            json!({ "project": dir, "clip": "c1", "scale": 0.5 }),
        ),
        (
            "clip_move",
            json!({ "project": dir, "clip": "c2", "start_seconds": 2.0 }),
        ),
    ];
    for (tool, arguments) in edits {
        let (text, failed) = said(&call(tool, arguments));
        assert!(!failed, "{tool}: {text}");
    }
    let c1 = member(&dir, "c1").expect("c1 is a member");
    assert!(
        c1["keyframes"].as_array().is_some_and(|k| !k.is_empty()),
        "clip_set scaled it"
    );
    assert_eq!(
        member(&dir, "c2").expect("c2 is a member")["start"],
        60,
        "clip_move moved it"
    );

    let (text, failed) = said(&call(
        "clip_remove",
        json!({ "project": dir, "clips": ["c2"] }),
    ));
    assert!(!failed, "{text}");
    assert!(member(&dir, "c2").is_none(), "clip_remove took it out");

    // clip_follow finds the member and goes on to the arrow it was not given.
    let (text, failed) = said(&call(
        "clip_follow",
        json!({ "project": dir, "clip": "c1", "arrow": "nowhere" }),
    ));
    assert!(failed && !text.contains("no clip `c1`"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn the_edits_a_member_cannot_take_name_its_group() {
    let dir = grouped("members-refused");
    let before = document(&dir);
    let refusals = [
        (
            "clip_move",
            json!({ "project": dir, "clip": "c1", "track": "v1" }),
        ),
        (
            "set_volume",
            json!({ "project": dir, "clip": "c1", "level": 0.5 }),
        ),
    ];
    for (tool, arguments) in refusals {
        let (text, failed) = said(&call(tool, arguments));
        assert!(failed, "{tool} refuses");
        assert!(
            text.contains("`c1` is inside group `titles`"),
            "{tool}: {text}"
        );
    }
    let after = document(&dir);
    assert_eq!(after, before, "nothing was written");
    std::fs::remove_dir_all(dir).ok();
}
