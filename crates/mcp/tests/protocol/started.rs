//! `project_new` with a platform and a style: the project starts with a brief
//! in its script, and the next step it names is proposing the script (#1015).

use serde_json::json;

use crate::starting::somewhere;
use crate::{call, said};

#[test]
fn a_platform_and_a_style_start_the_project_with_a_brief() {
    let dir = somewhere("started");
    let (text, failed) = said(&call(
        "project_new",
        json!({ "project": dir, "platform": "instagram_reels", "style": "kinetic_type" }),
    ));
    assert!(!failed, "{text}");
    assert!(
        text.contains("script.md"),
        "the reply names the brief: {text}"
    );

    let (brief, failed) = said(&call("script_read", json!({ "project": dir })));
    assert!(!failed, "{brief}");
    assert!(brief.contains("--platform instagram_reels"), "{brief}");
    assert!(brief.contains("Kinetic typography"), "{brief}");
    assert!(brief.contains("propose the script"), "{brief}");
    std::fs::remove_dir_all(dir).ok();
}

/// No platform and no style is the call as it always was: no script at all.
#[test]
fn without_either_no_script_is_written() {
    let dir = somewhere("unstarted");
    let (text, failed) = said(&call("project_new", json!({ "project": dir })));
    assert!(!failed, "{text}");
    assert!(!dir.join("script.md").exists(), "a stub was written");
    let document = std::fs::read_to_string(dir.join("project.json")).expect("read the document");
    assert!(!document.contains("\"script\""), "got {document}");
    std::fs::remove_dir_all(dir).ok();
}

/// A typo in either is refused with what would have been accepted, before a
/// single file is written.
#[test]
fn an_unknown_platform_or_style_is_refused_and_nothing_is_made() {
    for (arguments, expected) in [
        (json!({ "platform": "myspace" }), "tiktok_ad"),
        (json!({ "style": "vlog" }), "kinetic_type"),
        (
            json!({ "platform": "tiktok_ad", "style": "whiteboard" }),
            "kinetic_type",
        ),
    ] {
        let dir = somewhere("refused");
        let mut arguments = arguments;
        arguments["project"] = json!(dir);
        let (text, failed) = said(&call("project_new", arguments));
        assert!(failed, "should have been refused: {text}");
        assert!(text.contains(expected), "got {text}");
        assert!(!dir.exists(), "a refusal made a directory anyway");
    }
}
