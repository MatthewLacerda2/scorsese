//! `caption_narration` putting a generated line's words on screen, timed from
//! the timings stored beside it (#965).

use scorsese_core::ProjectPath;
use scorsese_core::words::{Word, Words};
use serde_json::{Value, json};

use super::fixture::project;
use crate::{call, said};

#[test]
fn a_timed_line_becomes_captions_that_a_rerun_replaces() {
    let dir = project("captioning");
    let document = std::fs::read_to_string(dir.join("project.json")).unwrap();
    let spoken = document.replace(
        r#""prompt": "a line", "state": "sketch""#,
        r#""prompt": "a line", "state": "generated", "path": "generated/vo.mp3""#,
    );
    std::fs::write(dir.join("project.json"), spoken).unwrap();
    std::fs::write(dir.join("generated/vo.mp3"), b"MP3").unwrap();
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
    std::fs::write(beside.resolve(&dir), timed.to_json()).unwrap();

    for _ in 0..2 {
        let (text, failed) = said(&call("caption_narration", json!({ "project": dir })));
        assert!(!failed, "{text}");
        assert!(
            text.starts_with("2 caption(s) from 1 line(s) on track `captions`"),
            "{text}"
        );
    }

    let document: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("project.json")).unwrap()).unwrap();
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
    std::fs::remove_dir_all(dir).ok();
}
