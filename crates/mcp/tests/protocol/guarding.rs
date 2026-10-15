//! The read/write pair, and what stops the second of two writers from taking
//! the first one's work with it.

use super::fixture::{DOCUMENT, document, fingerprint, project};
use crate::{call, said};
use serde_json::{Value, json};

/// Two callers, one document. The one that read first is refused, and told
/// what to do rather than left thinking its edit landed.
#[test]
fn a_write_built_on_a_document_something_else_replaced_is_refused() {
    let dir = project("guard-stale");
    let stale = fingerprint(&dir);

    let (text, failed) = said(&call(
        "project_write",
        json!({
            "project": dir,
            "document": DOCUMENT.replace("Teaser", "Somebody Else"),
            "fingerprint": stale.clone(),
        }),
    ));
    assert!(!failed, "the first writer publishes: {text}");
    let landed = document(&dir);

    let (text, failed) = said(&call(
        "project_write",
        json!({
            "project": dir,
            "document": DOCUMENT.replace("Teaser", "Trailer"),
            "fingerprint": stale,
        }),
    ));

    assert!(failed, "a stale write must be refused: {text}");
    assert!(text.contains("nothing written"), "got {text}");
    assert!(
        text.to_lowercase().contains("read the project again"),
        "got {text}"
    );
    assert_eq!(
        document(&dir),
        landed,
        "the first writer's document is still the one on disk"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// No fingerprint is not a shortcut: a document nobody says they read is a
/// document that would overwrite whatever is there blind.
#[test]
fn a_write_with_no_fingerprint_is_refused() {
    let dir = project("guard-none");
    let before = document(&dir);

    let (text, failed) = said(&call(
        "project_write",
        json!({ "project": dir, "document": DOCUMENT.replace("Teaser", "Trailer") }),
    ));

    assert!(failed, "a write with nothing to compare must be refused");
    assert!(text.contains("nothing written"), "got {text}");
    assert_eq!(document(&dir), before, "the file on disk must be untouched");
    std::fs::remove_dir_all(dir).ok();
}

/// The document has to arrive as the file and nothing else — a client that
/// parses what it was handed still gets a project.
#[test]
fn the_document_and_the_fingerprint_are_separate_blocks() {
    let dir = project("guard-blocks");
    let reply = call("project_read", json!({ "project": dir }));

    let (document, failed) = said(&reply);
    assert!(!failed, "{document}");
    serde_json::from_str::<Value>(&document).expect("the first block is the document itself");
    assert!(!fingerprint(&dir).is_empty());
    std::fs::remove_dir_all(dir).ok();
}

/// The read is compact and the file is not, so the fingerprint is of the file:
/// what project_read hands over, written straight back with its fingerprint,
/// is accepted — and the file keeps the layout `Project::save` gives it (#828).
#[test]
fn a_compact_read_written_back_is_accepted_and_saved_readable() {
    let dir = project("guard-compact");
    let reply = call("project_read", json!({ "project": dir }));
    let (compact, _) = said(&reply);
    assert!(!compact.contains('\n'), "read compact: {compact}");

    let (text, failed) = said(&call(
        "project_write",
        json!({ "project": dir, "document": compact, "fingerprint": fingerprint(&dir) }),
    ));
    assert!(!failed, "{text}");
    let saved = document(&dir);
    assert!(saved.contains("\n  "), "saved indented: {saved}");
    std::fs::remove_dir_all(dir).ok();
}
