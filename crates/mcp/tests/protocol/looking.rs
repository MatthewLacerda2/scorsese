//! The tools that only read: listing, describing, checking, and the pool.

use super::fixture::{DOCUMENT, EMPTY_SHA256, project, with_footage};
use crate::{call, once, said};
use serde_json::json;

#[test]
fn listing_tools_gives_a_name_a_description_and_a_schema_for_each() {
    let reply = once(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }).to_string());
    let tools = reply["result"]["tools"].as_array().expect("a list");
    assert!(!tools.is_empty());
    for tool in tools {
        assert!(tool["name"].as_str().is_some_and(|n| !n.is_empty()));
        assert!(tool["description"].as_str().is_some_and(|d| d.len() > 20));
        assert_eq!(tool["inputSchema"]["type"], json!("object"));
    }
}

#[test]
fn reading_a_project_gives_back_the_document() {
    let dir = project("read");
    let (text, failed) = said(&call("project_read", json!({ "project": dir })));
    assert!(!failed, "{text}");
    assert!(text.contains("\"name\": \"Teaser\""), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn describing_a_project_says_what_is_in_the_cut() {
    let dir = project("describe");
    let (text, failed) = said(&call("project_describe", json!({ "project": dir })));
    assert!(!failed, "{text}");
    assert!(text.contains("Teaser"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn checking_a_healthy_project_says_so() {
    let dir = project("check");
    let (text, failed) = said(&call("project_check", json!({ "project": dir })));
    assert!(!failed && text.contains("no problems"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// Being asked what is wrong and finding something is this tool working, so it
/// comes back as an answer rather than as an error.
#[test]
fn checking_a_broken_project_reports_the_problems_as_an_answer() {
    let dir = project("check-broken");
    let broken = DOCUMENT.replace(r#""asset": "bed""#, r#""asset": "ghost""#);
    std::fs::write(dir.join("project.json"), broken).expect("write");

    let (text, failed) = said(&call("project_check", json!({ "project": dir })));
    assert!(!failed, "a report of faults is not itself a fault");
    assert!(text.contains("ghost"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// Why #334 was filed: a project whose JSON is flawless and whose footage was
/// deleted underneath it used to be told there was nothing wrong with it.
#[test]
fn checking_reports_a_file_the_document_names_and_the_disk_has_not_got() {
    let dir = project("check-media");
    std::fs::write(dir.join("project.json"), with_footage("")).expect("write");

    let (text, failed) = said(&call("project_check", json!({ "project": dir })));
    assert!(!failed, "a report of faults is not itself a fault");
    assert!(text.contains("boat"), "got {text}");
    assert!(text.contains("its file is not there"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// Hashing is opt-in, and both answers are right: media edited behind the
/// project's back is worth knowing about, and re-reading a whole pool on a
/// tool called after every edit is not worth paying for.
#[test]
fn bytes_that_changed_since_import_are_found_only_when_asked_for() {
    let dir = project("check-verify");
    let recorded = format!(r#", "sha256": "{EMPTY_SHA256}""#);
    std::fs::write(dir.join("project.json"), with_footage(&recorded)).expect("write");
    std::fs::write(dir.join("assets/boat.mp4"), b"not what was imported").expect("write");

    let (quiet, _) = said(&call("project_check", json!({ "project": dir })));
    assert!(!quiet.contains("changed since import"), "got {quiet}");

    let asked = call("project_check", json!({ "project": dir, "verify": true }));
    let (verified, failed) = said(&asked);
    assert!(!failed, "{verified}");
    assert!(verified.contains("changed since import"), "got {verified}");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn listing_assets_names_them_and_their_state() {
    let dir = project("assets");
    let (text, failed) = said(&call("project_assets", json!({ "project": dir })));
    assert!(!failed, "{text}");
    assert!(text.contains("bed") && text.contains("vo"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}
