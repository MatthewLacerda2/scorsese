//! `look` on the wire: a video file in, one labelled sheet back.
//!
//! What is asserted here is the *shape of the reply*, as in `seeing`: a client
//! that cannot see the picture cannot use this tool at all, and a client with
//! no vision still has to get an answer it can read. The pixels themselves are
//! `scorsese-render`'s tests.

use crate::{call, said};
use scorsese_render::Tools;
use serde_json::{Value, json};

/// What the base64 of a PNG starts with, whatever is in the picture.
const PNG_HEADER: &str = "iVBORw0KGgo";

/// A short video on disk, outside any project — which is half the point of
/// this tool.
fn footage(label: &str, seconds: u32) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("scorsese-look-{label}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create the fixture directory");
    let file = dir.join("footage.mp4");
    let tools = Tools::discover().expect("ffmpeg must be on PATH to run these tests");
    let output = tools
        .ffmpeg()
        .args(["-nostdin", "-v", "error", "-y", "-f", "lavfi", "-i"])
        .arg(format!("color=c=teal:s=64x32:d={seconds}:r=10"))
        .args(["-pix_fmt", "yuv420p"])
        .arg(&file)
        .output()
        .expect("run ffmpeg");
    assert!(output.status.success(), "ffmpeg failed making the fixture");
    file
}

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
        .unwrap_or_else(|| panic!("no picture in {reply}"))
}

#[test]
fn a_sheet_comes_back_as_a_png_with_the_moments_named_in_words() {
    let file = footage("wire", 12);
    let reply = call("look", json!({ "project": ".", "file": file }));
    let (text, failed) = said(&reply);
    assert!(!failed, "{text}");
    assert!(
        text.contains("0:00") && text.contains("0:10"),
        "a client with no vision still needs the moments: {text}"
    );

    let image = pictured(&reply);
    assert_eq!(image["type"], "image");
    assert_eq!(image["mimeType"], "image/png");
    assert!(
        image["data"]
            .as_str()
            .is_some_and(|data| data.starts_with(PNG_HEADER)),
        "that is not the base64 of a PNG"
    );
    std::fs::remove_dir_all(file.parent().expect("a parent")).ok();
}

/// Walking a long file is the expected use, so where to carry on from is said
/// rather than left to be worked out.
#[test]
fn a_file_longer_than_one_sheet_says_where_to_carry_on() {
    let file = footage("walk", 40);
    let (text, failed) = said(&call("look", json!({ "project": ".", "file": file })));
    assert!(!failed, "{text}");
    assert!(text.contains("from: 25"), "no next step named: {text}");

    let (text, failed) = said(&call(
        "look",
        json!({ "project": ".", "file": file, "from": 25 }),
    ));
    assert!(!failed, "{text}");
    assert!(
        text.contains("whole file"),
        "the second pass reaches the end: {text}"
    );
    std::fs::remove_dir_all(file.parent().expect("a parent")).ok();
}

/// The cap is told, never silently applied. A limit that quietly rewrites the
/// request teaches nobody anything.
#[test]
fn asking_for_more_than_a_sheet_holds_is_refused_with_the_number() {
    let file = footage("cap", 40);
    let (text, failed) = said(&call(
        "look",
        json!({ "project": ".", "file": file, "frames": 12 }),
    ));
    assert!(failed, "12 frames must be refused, and got: {text}");
    assert!(text.contains('5'), "the cap is not named: {text}");
    std::fs::remove_dir_all(file.parent().expect("a parent")).ok();
}

/// A file that is not there, and a range that runs backwards. Both are answers
/// in words, and neither carries a picture of something that did not happen.
#[test]
fn a_missing_file_and_a_backwards_range_are_refused_in_words() {
    let (text, failed) = said(&call(
        "look",
        json!({ "project": ".", "file": "nowhere/at/all.mp4" }),
    ));
    assert!(failed, "a missing file must be refused, and got: {text}");

    let file = footage("backwards", 12);
    let reply = call(
        "look",
        json!({ "project": ".", "file": file, "from": 8, "to": 2 }),
    );
    let (text, failed) = said(&reply);
    assert!(failed, "a backwards range must be refused, and got: {text}");
    assert_eq!(
        reply["result"]["content"]
            .as_array()
            .expect("content")
            .len(),
        1,
        "a refusal must not carry a picture"
    );
    std::fs::remove_dir_all(file.parent().expect("a parent")).ok();
}
