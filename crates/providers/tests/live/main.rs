//! The live provider check's logic, against fixtures — never a real call.
//!
//! The check itself spends money and is run by a person (`scorsese
//! check-providers`, docs/live-check.md). What can be proved without a
//! network is everything it *decides*: what a reply means, what a plan costs,
//! whether the ceiling lets it run, and what a recording keeps. Each judge is
//! fed the bodies the product's own serde types parse — captured ones where a
//! vendor's real body exists, hand-written ones (marked so in the file) where
//! it does not yet.

#[path = "../common/mod.rs"]
mod common;

mod claude;
mod elevenlabs;
mod planning;
mod recording;
mod veo;

use scorsese_providers::api::http::HttpError;

/// A fixture's text, by its path under `fixtures/`.
fn fixture(path: &str) -> String {
    let path = format!("{}/fixtures/{path}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

/// A fixture, parsed as the client parses it.
fn parsed<T: serde::de::DeserializeOwned>(path: &str) -> T {
    serde_json::from_str(&fixture(path)).expect("the fixture parses as the client parses it")
}

/// A vendor refusal with this status and body.
fn refused(status: u16, body: &str) -> HttpError {
    HttpError::Refused {
        url: String::from("https://vendor.invalid/"),
        status,
        body: body.to_owned(),
    }
}
