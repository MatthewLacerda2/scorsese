//! `describe --at` names the word a narration is saying (#1289), as
//! `project_describe` does: sync checked without listening, from either
//! surface.

use scorsese_core::ProjectPath;
use scorsese_core::words::{Word, Words};

use super::{describe, project};

fn word(text: &str, start: f64, end: f64) -> Word {
    Word {
        text: text.into(),
        start,
        end,
    }
}

#[test]
fn at_names_the_word_each_narration_is_saying() {
    let dir = project("words");
    let document = std::fs::read_to_string(dir.join("project.json")).expect("read the document");
    std::fs::write(
        dir.join("project.json"),
        document.replace(
            r#""state": "sketch", "prompt": "a voice, later""#,
            r#""state": "generated", "prompt": "a voice, later", "path": "generated/vo.mp3""#,
        ),
    )
    .expect("write the document");
    std::fs::create_dir_all(dir.join("generated")).expect("create generated/");
    std::fs::write(dir.join("generated/vo.mp3"), b"MP3").expect("write the line");
    let timed = Words {
        words: vec![word("A", 0.2, 0.4), word("voice.", 0.5, 1.0)],
    };
    let beside = Words::beside(&ProjectPath::new("generated/vo.mp3"));
    std::fs::write(beside.resolve(&dir), timed.to_json()).expect("write the timings");

    let (succeeded, output) = describe(&dir, &["--at", "0.7s", "--at", "0.45s"]);

    assert!(succeeded, "{output}");
    assert!(
        output.contains(r#"vo1 is saying "voice." — scorsese.words["vo1/voice"], 0.50s to 1.00s"#),
        "{output}"
    );
    assert!(output.contains("vo1 is between words"), "{output}");
    std::fs::remove_dir_all(&dir).ok();
}
