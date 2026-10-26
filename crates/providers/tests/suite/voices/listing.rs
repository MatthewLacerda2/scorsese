//! The listing a reader is shown: every voice, its description under it, and
//! where the list came from — the one text the CLI and MCP both print.

use scorsese_providers::voices::builtin;

use super::fake::Fake;
use super::{root, voice};

#[test]
fn each_voice_has_its_description_under_it_then_the_summary() {
    let dir = root("voices-says");
    let answer = builtin(&dir, &Fake::listing(vec![voice("v1", "Ana")]), false).expect("listed");

    let said = answer.says(false, "--refresh");

    let lines: Vec<&str> = said.lines().collect();
    assert_eq!(lines[0], answer.voices[0].says());
    assert_eq!(lines[1], "    warm, unhurried");
    assert_eq!(lines[2], "", "a blank line before where it came from");
    assert_eq!(
        said.split_once("\n\n").map(|(_, tail)| tail.to_owned()),
        Some(answer.summary("built-in", "--refresh"))
    );
}

#[test]
fn an_empty_listing_says_nothing_matched_in_which_list() {
    let dir = root("voices-says-none");
    let answer = builtin(&dir, &Fake::listing(Vec::new()), false).expect("listed");

    let said = answer.says(true, "refresh: true");

    assert!(
        said.starts_with("No voices matched in the Voice Library.\n\n"),
        "{said}"
    );
}
