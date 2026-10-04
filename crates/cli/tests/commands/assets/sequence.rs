//! A sequence's stills are listed under it, collapsed (#684): one line for a
//! timelapse, with only the stills worth a look named beneath it.

use super::{list, row_for};
use crate::common::documents::{self, EMPTY_SHA256};

/// A probed, hashed still at `assets/{id}.png`.
fn still(id: &str) -> String {
    format!(
        r#"{{ "id": "{id}", "kind": "image", "path": "assets/{id}.png",
              "sha256": "{EMPTY_SHA256}", "media": {{ "width": 64, "height": 64 }} }}"#
    )
}

#[test]
fn a_sequence_is_one_line_naming_only_the_stills_that_need_a_look() {
    let dir = documents::project(
        "sequence-listing",
        &[
            still("quiet"),
            still("gone"),
            still("shared"),
            r#"{ "id": "spin", "kind": "image_sequence",
                 "sequence": { "stills": ["quiet", "gone", "shared"] } }"#
                .to_owned(),
        ],
        &[documents::clip("c1", "shared", 0)],
    );
    documents::file_at(&dir, "assets/quiet.png", b"");
    documents::file_at(&dir, "assets/shared.png", b"");
    let run = list(&dir, &[]).ok();

    assert!(row_for(&run, "spin").contains("3 stills"));
    assert!(
        !run.output.lines().any(|line| line.contains("quiet")),
        "a healthy still nobody shows alone stays folded:\n{}",
        run.output
    );
    run.says("└ gone");
    run.says("FILE MISSING");
    run.says("(also used on its own)");
    // `gc` keeps a still a sequence plays, so the tally does not call it unused.
    run.says("4 assets, 1 unused, 1 needing attention");
    std::fs::remove_dir_all(&dir).ok();
}
