//! Where a file a tool writes lands when its path is relative.
//!
//! The server's working directory belongs to whoever launched it — here, the
//! test binary's — so a relative `out` resolved against it lands somewhere the
//! caller never named, and the next call, which resolves the same string
//! against the project, cannot open it (#518, and #496 before it for
//! `synth_bake`). Each test asserts both halves: the file is inside the
//! project, and nothing appeared beside the server.

use std::path::Path;

use super::fixture::project;
use crate::{call, said};
use serde_json::json;

/// A render's `out`, and the format read off it: the extension still chooses
/// the container once the path has been resolved.
#[test]
fn a_relative_render_out_lands_in_the_project() {
    let dir = project("land-render");
    let out = "land-render-518.wav";
    let (text, failed) = said(&call(
        "render",
        json!({ "project": dir, "out": out, "range": "0:3" }),
    ));
    assert!(!failed, "render refused: {text}");
    assert!(
        text.contains(&format!("wrote {out}")),
        "the caller's path is said back: {text}"
    );
    assert!(
        text.contains("as wav"),
        "the extension chose the format: {text}"
    );
    assert!(dir.join(out).is_file(), "it lands inside the project");
    assert!(
        !Path::new(out).exists(),
        "and nothing lands in the server's working directory"
    );

    let level = said(&call("audio_level", json!({ "project": dir, "file": out })));
    assert!(!level.1, "the path the reply named reads back: {}", level.0);
    std::fs::remove_dir_all(dir).ok();
}

/// A kept still's `out`, under a folder of the project's.
#[test]
fn a_relative_still_out_lands_in_the_project() {
    let dir = project("land-still");
    std::fs::create_dir_all(dir.join("review")).expect("make the review folder");
    let out = "review/land-still-518.png";
    let (text, failed) = said(&call(
        "still",
        json!({ "project": dir, "at": "0", "resolution": "160x90", "out": out }),
    ));
    assert!(!failed, "{text}");
    assert!(
        text.contains(&format!("written to {out}")),
        "the caller's path is said back: {text}"
    );
    assert!(dir.join(out).is_file(), "it lands inside the project");
    assert!(
        !Path::new(out).exists(),
        "and nothing lands in the server's working directory"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// An empty `out` names nothing, and is refused rather than read as the
/// project directory itself.
#[test]
fn an_empty_out_is_refused() {
    let dir = project("land-empty");
    for tool in ["render", "still"] {
        let (text, failed) = said(&call(
            tool,
            json!({ "project": dir, "at": "0", "out": " " }),
        ));
        assert!(failed, "{tool} must refuse an empty out, and got: {text}");
        assert!(text.contains("`out` is empty"), "{tool}: {text}");
    }
    std::fs::remove_dir_all(dir).ok();
}
