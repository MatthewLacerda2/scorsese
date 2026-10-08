//! `scorsese guide` answers as the MCP tool does, and the CLI's own pointers
//! to more reading name it rather than a path in this repository (#916).
//!
//! A message that says "see docs/prices.md" helps only somebody sitting in a
//! checkout; an installed build has no `docs/`. The pointers are found by
//! reading the CLI's source — help text and printed messages alike — so one
//! added tomorrow is held to the same rule, and every one is followed, so a
//! pointer to a guide or a section that is not there fails here rather than
//! sending its reader to a refusal.

use std::path::Path;
use std::process::Command;

use scorsese_core::guide;

/// Runs `scorsese guide` with these arguments: whether it succeeded, and what
/// it printed on standard output.
fn run(arguments: &[&str]) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_scorsese"))
        .arg("guide")
        .args(arguments)
        .output()
        .expect("run scorsese guide");
    (
        output.status.success(),
        String::from_utf8(output.stdout).expect("utf-8"),
    )
}

#[test]
fn it_prints_what_the_tool_answers() {
    let (ok, printed) = run(&["pages"]);
    assert!(ok);
    assert_eq!(
        printed,
        format!("{}\n", guide::read("pages", None).unwrap())
    );
    let (ok, printed) = run(&["recipes", "--section", "2"]);
    assert!(ok);
    assert_eq!(
        printed,
        format!("{}\n", guide::read("recipes", Some("2")).unwrap())
    );
}

#[test]
fn a_guide_that_is_not_there_fails_and_says_what_is() {
    let output = Command::new(env!("CARGO_BIN_EXE_scorsese"))
        .args(["guide", "tutorial"])
        .output()
        .expect("run scorsese guide");
    assert!(!output.status.success());
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(said.contains("pages, project-format"), "{said}");
}

/// Every `.rs` file under `dir`, with its text.
fn sources(dir: &Path, found: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(dir).expect("read the source tree") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let text = std::fs::read_to_string(&path).expect("read a source file");
            found.push((path.display().to_string(), text));
        }
    }
}

/// The guide a pointer names, and the section after `--section` if it gives
/// one — or nothing, for a placeholder like `<name>`.
fn pointer(after: &str) -> Option<(&str, Option<&str>)> {
    let name = word(after);
    if name.is_empty() {
        return None;
    }
    let section = after[name.len()..].strip_prefix(" --section ").map(word);
    Some((name, section))
}

/// The guide name or section number at the start of `text`.
fn word(text: &str) -> &str {
    let end = text
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .unwrap_or(text.len());
    &text[..end]
}

#[test]
fn every_pointer_to_reading_is_a_guide_that_answers() {
    let mut found = Vec::new();
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut found,
    );
    let mut followed = 0;
    for (file, text) in &found {
        for name in guide::names() {
            assert!(
                !text.contains(&format!("docs/{name}.md")),
                "{file} points at docs/{name}.md; name `scorsese guide {name}` instead"
            );
        }
        for after in text.split("scorsese guide ").skip(1) {
            let Some((name, section)) = pointer(after) else {
                continue;
            };
            if let Err(why) = guide::read(name, section) {
                panic!("{file} points at guide {name} {section:?}: {why}");
            }
            followed += 1;
        }
    }
    assert!(followed > 0, "the pointers were found");
}
