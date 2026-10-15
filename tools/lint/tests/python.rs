//! Python under the gate (#1136): which files are measured, and what in them
//! counts. The rule is Rust's — blank lines and prose are free — so these pin
//! the two shapes Python's prose takes, `#` comments and docstrings, and the
//! one way a docstring can be mistaken for: a triple-quoted string of data.

use std::fs;
use std::path::Path;

use scorsese_lint::classify::{Kind, classify};
use scorsese_lint::{check_dir, python::code_lines};

fn count(text: &str) -> usize {
    code_lines(text.as_bytes())
}

#[test]
fn a_script_is_source_and_a_test_is_a_test() {
    let kind_of = |path: &str| classify(Path::new(path));
    assert_eq!(
        kind_of(".github/scripts/merge-queue.py"),
        Some(Kind::Source)
    );
    assert_eq!(
        kind_of(".github/scripts/merge_queue/ci.py"),
        Some(Kind::Source)
    );
    assert_eq!(
        kind_of(".github/scripts/tests/queueing/test_queue.py"),
        Some(Kind::Test)
    );
    // The name alone makes a test, wherever it sits — and it is the prefix,
    // not the word.
    assert_eq!(kind_of("tools/test_thing.py"), Some(Kind::Test));
    assert_eq!(kind_of("tools/latest_test.py"), Some(Kind::Source));
}

#[test]
fn comments_and_blank_lines_are_free() {
    assert_eq!(
        count("#!/usr/bin/env python3\n\n# why\n    # and why\nx = 1\n"),
        1
    );
    assert_eq!(count("x = 1  # one\n"), 1);
}

#[test]
fn a_docstring_is_free_in_every_shape() {
    assert_eq!(count("\"\"\"One line.\"\"\"\nx = 1\n"), 1);
    assert_eq!(
        count("def f():\n    \"\"\"Why.\n\n    At length.\n    \"\"\"\n    return 1\n"),
        2
    );
    assert_eq!(
        count("'''Single quotes.\n'''\nr\"\"\"Raw.\"\"\"\nx = 1\n"),
        1
    );
    // Code after the docstring closes, on the same line, still counts.
    assert_eq!(count("\"\"\"Doc.\n\"\"\"; x = 1\n"), 1);
}

#[test]
fn a_string_opened_after_code_is_data_to_its_close() {
    // Every line of it counts, and a line inside it that starts with `"""` or
    // `#` is part of the string — not a docstring opening, not a comment.
    let text = "HELP = \"\"\"\n# usage\nmore\n\"\"\"\nx = 1\n";
    assert_eq!(count(text), 5);
    assert_eq!(count("X = \"\"\"a\"\"\"\n\"\"\"doc\"\"\"\n"), 1);
    // And it does close: what follows is read as prose and code again.
    assert_eq!(
        count("X = \"\"\"\nfoo\n\"\"\"\n# why\n\"\"\"Doc.\"\"\"\nx = 1\n"),
        4
    );
}

#[test]
fn a_name_starting_like_a_string_prefix_is_code() {
    // `b`, `r`, `f`… begin plenty of identifiers; only a quote after them
    // makes a prefix.
    assert_eq!(count("bar = 1\nfoo()\nrun()\n"), 3);
}

#[test]
fn the_scan_walks_github_scripts_and_counts_python_as_python() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("python");
    let _ = fs::remove_dir_all(&root);
    let scripts = root.join(".github/scripts/tests");
    fs::create_dir_all(&scripts).expect("mkdir");
    // 301 lines of code under a docstring that would put any count over.
    let long = format!(
        "\"\"\"{}\"\"\"\n{}",
        "prose\n".repeat(400),
        "x = 1\n".repeat(301)
    );
    fs::write(root.join(".github/scripts/long.py"), long).expect("write");
    fs::write(scripts.join("test_long.py"), "x = 1\n".repeat(151)).expect("write");
    fs::write(root.join(".github/scripts/short.py"), "# x\n".repeat(400)).expect("write");

    let report = check_dir(&root).expect("scan");

    assert_eq!(report.checked, 3);
    assert!(!report.is_clean());
    let over: Vec<_> = report
        .violations
        .iter()
        .map(|v| (v.lines, v.kind))
        .collect();
    assert_eq!(over, [(301, Kind::Source), (151, Kind::Test)]);
}
