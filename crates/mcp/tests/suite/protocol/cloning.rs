//! `voice_clone` asks for consent before anything else (#1263). Only the
//! halves that stop before a key is resolved are driven here, for
//! `paying`'s reason; what a clone does is pinned in `scorsese-providers`.

use super::fixture::project;
use crate::protocol::{call, said};
use serde_json::json;

#[test]
fn without_consent_nothing_is_sent_and_the_terms_are_named() {
    let dir = project("clone-consent");

    let (text, failed) = said(&call(
        "voice_clone",
        json!({ "project": dir, "assets": ["vo"], "name": "Me" }),
    ));

    assert!(failed, "{text}");
    assert!(text.contains("your own voice"), "{text}");
    assert!(
        text.contains("https://elevenlabs.io/terms-of-use"),
        "{text}"
    );
    assert!(!dir.join("cloned-voices.json").exists());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_project_that_cloned_nothing_lists_nothing_offline() {
    let dir = project("clone-list");

    let (text, failed) = said(&call(
        "voice_clone",
        json!({ "project": dir, "list": true }),
    ));

    assert!(!failed, "{text}");
    assert!(text.contains("No voices have been cloned"), "{text}");
    std::fs::remove_dir_all(dir).ok();
}
