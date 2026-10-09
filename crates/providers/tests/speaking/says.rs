//! The sentence each line outcome reads as — pinned word for word, because the
//! CLI and the MCP server both print it.

use scorsese_core::ProjectPath;
use scorsese_providers::speech::Outcome;

#[test]
fn every_line_outcome_reads_as_its_sentence() {
    let path = || ProjectPath::new("generated/vo-abc.mp3");
    let cases = [
        (
            Outcome::Cached { path: path() },
            "already spoken — generated/vo-abc.mp3",
        ),
        (
            Outcome::Generated {
                path: path(),
                bytes: 1234,
                estimated_cost_cents: 2,
            },
            "spoken — generated/vo-abc.mp3 (1234 bytes)",
        ),
        (
            Outcome::Incomplete {
                why: String::from("A line with no voice"),
            },
            "not yet — A line with no voice",
        ),
        (
            Outcome::Failed {
                message: String::from("no"),
            },
            "refused — no",
        ),
    ];
    for (outcome, sentence) in cases {
        assert_eq!(outcome.says(), sentence);
    }
}

#[test]
fn a_spoken_line_names_where_its_word_timings_were_saved() {
    let root = std::env::temp_dir().join(format!("scorsese-says-in-{}", std::process::id()));
    std::fs::create_dir_all(root.join("generated")).unwrap();
    let outcome = Outcome::Cached {
        path: ProjectPath::new("generated/vo-abc.mp3"),
    };
    assert_eq!(
        outcome.says_in(&root),
        outcome.says(),
        "no file, nothing claimed"
    );
    std::fs::write(root.join("generated/vo-abc.words.json"), "{}").unwrap();
    assert_eq!(
        outcome.says_in(&root),
        "already spoken — generated/vo-abc.mp3; word timings in generated/vo-abc.words.json"
    );
    std::fs::remove_dir_all(root).ok();
}
