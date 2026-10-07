//! `project_describe` naming the word a narration is saying (#811), so sync
//! can be checked without listening.

use scorsese_core::ProjectPath;
use scorsese_core::words::{Word, Words};
use serde_json::json;

use super::fixture::project;
use crate::{call, said};

fn word(text: &str, start: f64, end: f64) -> Word {
    Word {
        text: text.into(),
        start,
        end,
    }
}

#[test]
fn describing_an_instant_names_the_word_being_said() {
    let dir = project("syncing");
    let document = std::fs::read_to_string(dir.join("project.json")).unwrap();
    let spoken = document.replace(
        r#""prompt": "a line", "state": "sketch""#,
        r#""prompt": "a line", "state": "generated", "path": "generated/vo.mp3""#,
    );
    std::fs::write(dir.join("project.json"), spoken).unwrap();
    std::fs::write(dir.join("generated/vo.mp3"), b"MP3").unwrap();
    let timed = Words {
        words: vec![word("A", 0.2, 0.4), word("line.", 0.5, 1.0)],
    };
    let beside = Words::beside(&ProjectPath::new("generated/vo.mp3"));
    std::fs::write(beside.resolve(&dir), timed.to_json()).unwrap();

    // The line starts at 2 s: `line.` is heard from 2.5 s to 3 s.
    let (text, failed) = said(&call(
        "project_describe",
        json!({ "project": dir, "at": ["2.7s", "2.45s", "1s"] }),
    ));
    assert!(!failed, "{text}");
    assert!(
        text.contains(r#"v1c is saying "line." — scorsese.words["v1c/line"], 2.50s to 3.00s"#),
        "{text}"
    );
    assert!(text.contains("v1c is between words"), "{text}");
    assert_eq!(text.matches("v1c is").count(), 2, "silent at 1 s: {text}");

    std::fs::remove_file(beside.resolve(&dir)).unwrap();
    let (text, _) = said(&call(
        "project_describe",
        json!({ "project": dir, "at": "2.7s" }),
    ));
    assert!(text.contains("v1c has no word timings"), "{text}");
    std::fs::remove_dir_all(dir).ok();
}
