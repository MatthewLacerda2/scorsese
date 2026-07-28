//! The tools that only read: listing, describing, checking, and the pool.

use super::fixture::{DOCUMENT, project};
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

#[test]
fn listing_assets_names_them_and_their_state() {
    let dir = project("assets");
    let (text, failed) = said(&call("project_assets", json!({ "project": dir })));
    assert!(!failed, "{text}");
    assert!(text.contains("bed") && text.contains("vo"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}
