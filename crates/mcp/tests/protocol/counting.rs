//! A caption that moves on its own: the reveal and the counter `text_new`
//! takes, each written into the document as the block it is.

use super::fixture::project;
use crate::{call, said};
use serde_json::json;

fn document(dir: &std::path::Path) -> serde_json::Value {
    let text = std::fs::read_to_string(dir.join("project.json")).expect("the project is on disk");
    serde_json::from_str(&text).expect("the project is JSON")
}

#[test]
fn a_counter_and_a_reveal_are_written_as_their_blocks() {
    let dir = project("text-counter");
    let (text, failed) = said(&call(
        "text_new",
        json!({ "project": dir, "text": "{n} partitions", "asset": "partitions",
                "reveal": { "unit": "char", "stagger": 1.0 },
                "number": { "value": 144, "locale": "pt-BR" } }),
    ));
    assert!(!failed, "{text}");
    let written = document(&dir);
    let style = written["assets"]
        .as_array()
        .and_then(|assets| assets.iter().find(|asset| asset["id"] == "partitions"))
        .map(|asset| asset["style"].clone())
        .expect("the asset was written");
    assert_eq!(style["reveal"]["unit"], "char");
    assert_eq!(style["number"]["locale"], "pt-BR");
    assert_eq!(style["number"]["value"], 144.0);
    std::fs::remove_dir_all(dir).ok();
}

/// The document's own grammar refuses what the document would: a counter with
/// nowhere to write its figure, and a unit that is not one.
#[test]
fn a_counter_without_a_placeholder_and_an_unknown_unit_are_refused() {
    let dir = project("text-counter-refused");
    let (text, failed) = said(&call(
        "text_new",
        json!({ "project": dir, "text": "partitions", "number": { "value": 144 } }),
    ));
    assert!(failed, "no `{{n}}` to write the figure at");
    assert!(text.contains("{n}"), "the reason names it: {text}");
    let (text, failed) = said(&call(
        "text_new",
        json!({ "project": dir, "text": "Ship it", "reveal": { "unit": "sentence" } }),
    ));
    assert!(failed, "`sentence` is not a unit");
    assert!(
        text.contains("reveal"),
        "the reason names the block: {text}"
    );
    std::fs::remove_dir_all(dir).ok();
}

fn style_of(dir: &std::path::Path, id: &str) -> serde_json::Value {
    document(dir)["assets"]
        .as_array()
        .and_then(|assets| assets.iter().find(|asset| asset["id"] == id))
        .map(|asset| asset["style"].clone())
        .expect("the asset is in the document")
}

/// `asset_set` changes one field of a block and keeps the rest, and `false`
/// takes a block away — "letter by letter, and count to 144" is one call.
#[test]
fn asset_set_merges_into_a_block_and_false_removes_it() {
    let dir = project("text-counter-set");
    let (text, failed) = said(&call(
        "text_new",
        json!({ "project": dir, "text": "{n} partitions", "asset": "partitions",
                "reveal": { "rise": 0 }, "number": { "value": 140, "locale": "pt-BR" } }),
    ));
    assert!(!failed, "{text}");
    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "asset": "partitions",
                "reveal": { "unit": "char" }, "number": { "value": 144 } }),
    ));
    assert!(!failed, "{text}");
    assert!(
        text.contains("by word, rise 0, stagger 0.5 → by char"),
        "{text}"
    );
    let style = style_of(&dir, "partitions");
    assert_eq!(style["reveal"]["unit"], "char");
    assert_eq!(style["reveal"]["rise"], 0.0, "the rise chosen stays");
    assert_eq!(style["number"]["value"], 144.0);
    assert_eq!(
        style["number"]["locale"], "pt-BR",
        "the locale chosen stays"
    );

    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "asset": "partitions", "reveal": false }),
    ));
    assert!(!failed, "{text}");
    assert!(style_of(&dir, "partitions").get("reveal").is_none());

    let (text, failed) = said(&call(
        "asset_set",
        json!({ "project": dir, "asset": "partitions", "number": true }),
    ));
    assert!(failed, "`true` is neither fields nor a removal");
    assert!(
        text.contains("`number`"),
        "the reason names the block: {text}"
    );
    std::fs::remove_dir_all(dir).ok();
}
