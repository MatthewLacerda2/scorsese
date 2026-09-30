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
