//! `caption_narration` putting a generated line's words on screen, timed from
//! the timings stored beside it (#965).

use super::fixture::{document, project, read};
use scorsese_core::ProjectPath;
use scorsese_core::words::{Word, Words};
use serde_json::{Value, json};

use crate::protocol::{call, said};

/// A project whose line `vo`, played by clip `v1c` from 2 s, says "Stop. Read
/// this." with its timings beside it.
fn timed(label: &str) -> std::path::PathBuf {
    let dir = project(label);
    let document = document(&dir);
    let spoken = document.replace(
        r#""prompt": "a line", "state": "sketch""#,
        r#""prompt": "a line", "state": "generated", "path": "generated/vo.mp3""#,
    );
    std::fs::write(dir.join("project.json"), spoken).expect("the document");
    std::fs::write(dir.join("generated/vo.mp3"), b"MP3").expect("the line");
    let word = |text: &str, start, end| Word {
        text: text.into(),
        start,
        end,
    };
    let timed = Words {
        words: vec![
            word("Stop.", 0.2, 0.6),
            word("Read", 0.9, 1.1),
            word("this.", 1.1, 1.5),
        ],
    };
    let beside = Words::beside(&ProjectPath::new("generated/vo.mp3"));
    std::fs::write(
        beside.resolve(&dir).expect("a project path"),
        timed.to_json(),
    )
    .expect("its timings");
    dir
}

#[test]
fn a_timed_line_becomes_captions_that_a_rerun_replaces() {
    let dir = timed("captioning");

    // The second run names the rim's colour, as `scorsese caption --stroke`
    // can (#1288).
    for asked in [
        json!({ "project": dir, "look": "text" }),
        json!({ "project": dir, "look": "text", "stroke": "#ff0000" }),
    ] {
        let (text, failed) = said(&call("caption_narration", asked));
        assert!(!failed, "{text}");
        assert!(
            text.starts_with("2 caption(s) from 1 line(s) on track `captions`"),
            "{text}"
        );
    }

    let document: Value = read(&dir);
    let track = document["tracks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|track| track["id"] == "captions")
        .expect("the captions' track was made");
    let clips = track["clips"].as_array().unwrap();
    assert_eq!(clips.len(), 2, "the second run replaced the first");
    // The line starts at 2 s on a 30 fps grid: "Stop." is heard at 2.2 s and
    // its caption arrives 0.07 s ahead, frame 64 (63.9).
    assert_eq!(clips[0]["start"], 64);
    let first = document["assets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|asset| asset["id"] == clips[0]["asset"])
        .unwrap();
    assert_eq!(first["text"], "Stop.");
    assert_eq!(first["style"]["font"], "montserrat");
    assert_eq!(first["style"]["stroke"], "#ff0000");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn by_default_the_captions_are_one_page_lighting_the_word_said() {
    let dir = timed("captioning-page");
    let (text, failed) = said(&call("caption_narration", json!({ "project": dir })));
    assert!(!failed, "{text}");
    assert!(text.contains("one page, `caption-page-captions`"), "{text}");
    let document: Value = read(&dir);
    let track = document["tracks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|track| track["id"] == "captions")
        .expect("the captions' track was made");
    let clips = track["clips"].as_array().unwrap();
    assert_eq!(clips.len(), 1, "one clip plays the page");
    assert_eq!(clips[0]["asset"], "caption-page-captions");
    let page = std::fs::read_to_string(dir.join("pages/caption-page-captions.html")).unwrap();
    assert!(
        page.contains(r#"[["v1c/stop","Stop."]],[["v1c/read","Read"],["v1c/this","this."]]"#),
        "{page}"
    );
    std::fs::remove_dir_all(dir).ok();
}
