//! The paid tools quote first, and spend only on the token they quoted.
//!
//! Only the halves that stop **before** a key is resolved are driven here: the
//! quote, and every refusal of a token. A confirmed run would reach for a key,
//! and this process's environment is the developer's own — a test that got
//! that far would be one typo away from spending somebody's money. What a good
//! token does is pinned in `scorsese-providers`' own tests instead.

use super::fixture::{DOCUMENT, project};
use crate::{call, said};
use serde_json::json;

/// The fixture, with its line given a voice — so it is a line that would be
/// spoken, and paid for, rather than one still waiting on a choice.
fn voiced(label: &str) -> std::path::PathBuf {
    let dir = project(label);
    let document = DOCUMENT.replace(
        r#""prompt": "a line", "state": "sketch""#,
        r#""prompt": "a line", "state": "sketch", "speech": { "voice_id": "a-voice" }"#,
    );
    assert_ne!(document, DOCUMENT, "the fixture no longer has the line");
    std::fs::write(dir.join("project.json"), document).expect("write the document");
    dir
}

/// The token a quote handed out, taken out of the reply the way a client would.
fn token(text: &str) -> String {
    let start = text
        .find("quote-")
        .unwrap_or_else(|| panic!("no token in {text}"));
    text[start..]
        .split('"')
        .next()
        .expect("the token is quoted")
        .to_owned()
}

#[test]
fn a_first_call_quotes_and_sends_nothing() {
    let dir = voiced("pay-quote");
    let before = std::fs::read_to_string(dir.join("project.json")).expect("read");

    let (text, failed) = said(&call("generate", json!({ "project": dir })));

    assert!(!failed, "{text}");
    assert!(text.contains("vo: $0.01"), "the line is priced: {text}");
    assert!(text.contains("Nothing has been sent"), "{text}");
    assert!(text.contains("confirm"), "it says how to go ahead: {text}");
    token(&text);
    assert_eq!(
        std::fs::read_to_string(dir.join("project.json")).expect("read"),
        before,
        "a quote writes nothing into the document"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_token_nobody_issued_is_refused() {
    let dir = voiced("pay-forged");
    let (text, failed) = said(&call(
        "generate",
        json!({ "project": dir, "confirm": "quote-0123456789abcdef01234567" }),
    ));
    assert!(failed, "a made-up token spent: {text}");
    assert!(text.contains("Nothing was sent"), "{text}");
    std::fs::remove_dir_all(dir).ok();
}

/// The binding, end to end over the protocol: quoted, then the brief edited,
/// then confirmed — and refused, because the yes was given to the old words.
#[test]
fn a_brief_edited_after_the_quote_is_refused() {
    let dir = voiced("pay-edited");
    let (quoted, _) = said(&call("generate", json!({ "project": dir })));
    let (text, failed) = said(&call(
        "rebrief",
        json!({ "project": dir, "asset": "vo", "prompt": "a much longer line than before" }),
    ));
    assert!(!failed, "{text}");

    let (text, failed) = said(&call(
        "generate",
        json!({ "project": dir, "confirm": token(&quoted) }),
    ));
    assert!(failed, "an edited brief spent on an old quote: {text}");
    assert!(text.contains("changed since it was quoted"), "{text}");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_voice_design_quotes_first_too() {
    let dir = project("pay-design");
    let (text, failed) = said(&call(
        "voice_design",
        json!({
            "project": dir,
            "prompt": "a calm narrator in her forties, unhurried and warm",
            "text": "The tide came in slowly that evening, and nobody on the beach thought \
                     to move until the light had gone entirely.",
        }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("charged once"), "{text}");
    token(&text);
    std::fs::remove_dir_all(dir).ok();
}
