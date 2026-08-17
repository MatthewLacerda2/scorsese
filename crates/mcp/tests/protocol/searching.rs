//! `icons`: finding a symbol by a word, over the protocol.
//!
//! The whole tool is one string in and one string out, so what is worth
//! asserting here is that the string that comes out is usable — a name a client
//! can write straight into an `icon` asset — and that the two answers nobody
//! designs for, a capped list and an empty one, arrive as answers rather than
//! as silence or an error.

use super::fixture::project;
use crate::{call, said};
use serde_json::json;

/// What the tool said, having refused to refuse.
fn found(query: &str) -> String {
    let dir = project("icons");
    let (text, failed) = said(&call("icons", json!({ "project": dir, "query": query })));
    assert!(!failed, "`{query}` was refused: {text}");
    text
}

/// The case the tool exists for: a word that is nowhere in the name of the icon
/// it should find.
#[test]
fn a_word_finds_the_icons_filed_under_it() {
    let said = found("cinema");
    assert!(said.contains("clapperboard"), "{said}");
}

/// A name already known comes back first, so confirming one is a glance rather
/// than a read.
#[test]
fn a_known_name_leads_the_answer() {
    let said = found("film");
    let names = said.split(": ").nth(1).unwrap_or_default();
    assert!(names.starts_with("film,"), "{said}");
}

/// Both numbers, so a capped answer is never read as the whole set.
#[test]
fn a_broad_word_says_how_many_matched_as_well_as_how_many_are_shown() {
    let said = found("arrow");
    assert!(said.contains("of which the first"), "{said}");
    assert!(said.contains("Narrow the word"), "{said}");
}

/// Zero is an answer. It says what was searched, so the next move is another
/// word rather than a bug report.
#[test]
fn a_word_that_matches_nothing_answers_rather_than_fails() {
    let said = found("qzxwvjkhgfdsplm");
    assert!(said.starts_with("No icon matches"), "{said}");
    assert!(said.contains("tags"), "{said}");
}

/// A call with nothing to search on is the one refusal here, and it says what
/// to send instead.
#[test]
fn a_missing_or_empty_query_is_refused_in_words() {
    let dir = project("icons-empty");
    for arguments in [
        json!({ "project": &dir }),
        json!({ "project": &dir, "query": "  " }),
    ] {
        let (text, failed) = said(&call("icons", arguments));
        assert!(failed, "an empty query was answered: {text}");
        assert!(text.contains("`query` is required"), "{text}");
    }
}
