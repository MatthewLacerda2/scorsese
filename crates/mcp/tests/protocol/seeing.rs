//! The one tool that answers with a picture.
//!
//! What is asserted here is the *wire*: that a frame comes back as an image
//! content block, base64 of a real PNG, alongside the words. A client that
//! cannot see the picture cannot use this tool at all, so the shape of the
//! reply is the feature — the pixels themselves are the CLI's tests and the
//! golden gate's.

use super::fixture::project;
use crate::{call, said};
use serde_json::{Value, json};

/// The image block of a reply, or a failure saying what came back instead.
fn pictured(reply: &Value) -> &Value {
    let content = reply["result"]["content"]
        .as_array()
        .unwrap_or_else(|| panic!("no content in {reply}"));
    assert_eq!(
        content[0]["type"], "text",
        "the words come first — a client renders blocks in order"
    );
    content
        .get(1)
        .unwrap_or_else(|| panic!("no second block in {reply}"))
}

/// Small, because every one of these composites for real.
const SMALL: &str = "160x90";

#[test]
fn a_still_comes_back_as_a_png_the_client_can_see() {
    let dir = project("still");
    let reply = call(
        "still",
        json!({ "project": dir, "at": "1.0s", "resolution": SMALL }),
    );
    let (text, failed) = said(&reply);
    assert!(!failed, "{text}");
    assert!(
        text.contains("frame 30"),
        "the words say which frame: {text}"
    );

    let image = pictured(&reply);
    assert_eq!(image["type"], "image");
    assert_eq!(image["mimeType"], "image/png");
    let data = image["data"].as_str().expect("base64 data");
    assert!(
        data.starts_with("iVBORw0KGgo"),
        "that is not the base64 of a PNG header: {}",
        &data[..data.len().min(24)]
    );
    std::fs::remove_dir_all(dir).ok();
}

/// Nothing is left behind when nobody asked for a file. The picture goes over
/// the wire; the scratch it was written to on the way is removed.
#[test]
fn a_still_leaves_nothing_on_disk_unless_a_path_was_named() {
    let dir = project("still-clean");
    let before = std::fs::read_dir(&dir).expect("read the project").count();
    said(&call(
        "still",
        json!({ "project": dir, "at": "0", "resolution": SMALL }),
    ));
    assert_eq!(
        std::fs::read_dir(&dir).expect("read the project").count(),
        before,
        "the project directory grew"
    );

    let (text, failed) = said(&call(
        "still",
        json!({ "project": dir, "at": "0", "resolution": SMALL, "out": dir.join("kept.png") }),
    ));
    assert!(!failed, "{text}");
    assert!(dir.join("kept.png").is_file(), "`out` was not kept: {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// An instant nobody can name, and one the edit does not reach. Both are
/// refusals with a reason, and a refusal carries words and no picture — there
/// is no image of something that did not happen.
#[test]
fn an_instant_that_is_not_one_is_refused_in_words() {
    let dir = project("still-refused");
    for at in ["2.5", "9000"] {
        let reply = call("still", json!({ "project": dir, "at": at }));
        let (text, failed) = said(&reply);
        assert!(failed, "`{at}` must be refused, and got: {text}");
        assert_eq!(
            reply["result"]["content"]
                .as_array()
                .map(Vec::len)
                .unwrap_or_default(),
            1,
            "a refusal must not carry a picture"
        );
    }
    std::fs::remove_dir_all(dir).ok();
}
