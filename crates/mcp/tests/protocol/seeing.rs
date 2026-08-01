//! The one tool that answers with a picture.
//!
//! What is asserted here is the *wire*: that a frame comes back as an image
//! content block, base64 of a real PNG, alongside the words — and that a list
//! of instants comes back as one such pair each, in the order asked. A client
//! that cannot see the picture cannot use this tool at all, so the shape of
//! the reply is the feature — the pixels themselves are the CLI's tests and
//! the golden gate's.

use super::fixture::project;
use crate::{call, said};
use serde_json::{Value, json};

/// The content blocks of a reply, in the order a client would render them.
fn blocks(reply: &Value) -> &Vec<Value> {
    reply["result"]["content"]
        .as_array()
        .unwrap_or_else(|| panic!("no content in {reply}"))
}

/// The image block of a reply, or a failure saying what came back instead.
fn pictured(reply: &Value) -> &Value {
    let content = blocks(reply);
    assert_eq!(
        content[0]["type"], "text",
        "the words come first — a client renders blocks in order"
    );
    content
        .get(1)
        .unwrap_or_else(|| panic!("no second block in {reply}"))
}

/// What the base64 of a PNG starts with, whatever is in the picture.
const PNG_HEADER: &str = "iVBORw0KGgo";

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
        data.starts_with(PNG_HEADER),
        "that is not the base64 of a PNG header: {}",
        &data[..data.len().min(24)]
    );
    std::fs::remove_dir_all(dir).ok();
}

/// Several sections of a cut is one question, and it is answered in one
/// exchange: a sentence then its picture, alternating, in the order the
/// instants were asked for rather than in timeline order.
#[test]
fn a_list_of_instants_answers_with_a_picture_each_in_the_order_asked() {
    let dir = project("still-several");
    let reply = call(
        "still",
        json!({ "project": dir, "at": ["1.0s", "0", "599"], "resolution": SMALL }),
    );
    let (text, failed) = said(&reply);
    assert!(!failed, "{text}");

    let content = blocks(&reply);
    assert_eq!(content.len(), 6, "three sentences and three pictures");
    for (index, frame) in ["frame 30", "frame 0", "frame 599"].into_iter().enumerate() {
        let words = &content[index * 2];
        assert!(
            words["text"]
                .as_str()
                .is_some_and(|said| said.contains(frame)),
            "block {index} should be about {frame}: {words}"
        );
        let picture = &content[index * 2 + 1];
        assert_eq!(picture["type"], "image", "no picture under {words}");
        assert!(
            picture["data"]
                .as_str()
                .is_some_and(|data| data.starts_with(PNG_HEADER)),
            "block {index} is not a PNG"
        );
    }
    std::fs::remove_dir_all(dir).ok();
}

/// One path names one file and several frames do not fit in it, so the pair is
/// refused rather than reinterpreted — and the refusal writes nothing, because
/// a half-honoured `out` is worse than none.
#[test]
fn several_instants_and_a_path_to_keep_them_at_is_refused() {
    let dir = project("still-several-out");
    let kept = dir.join("kept.png");
    let reply = call(
        "still",
        json!({ "project": dir, "at": ["0", "30"], "resolution": SMALL, "out": kept }),
    );
    let (text, failed) = said(&reply);
    assert!(failed, "`out` with a list must be refused, and got: {text}");
    assert!(
        text.contains("--stills"),
        "the refusal must say where a set of PNGs comes from: {text}"
    );
    assert!(!kept.exists(), "a refused call left a file behind");
    assert_eq!(
        blocks(&reply).len(),
        1,
        "a refusal must not carry a picture"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// A list with nothing in it names no instant, and answering it with an empty
/// reply would read as "nothing to see" rather than as the mistake it is.
#[test]
fn an_empty_list_of_instants_is_refused() {
    let dir = project("still-none");
    let (text, failed) = said(&call("still", json!({ "project": dir, "at": [] })));
    assert!(failed, "an empty list must be refused, and got: {text}");
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
            blocks(&reply).len(),
            1,
            "a refusal must not carry a picture"
        );
    }
    std::fs::remove_dir_all(dir).ok();
}
