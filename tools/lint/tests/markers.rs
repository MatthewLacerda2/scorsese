//! Nothing temporary: a marker promising later work is flagged, a word that
//! merely contains one is not.

use std::path::Path;

use scorsese_lint::rules::markers::{MARKERS, RULE, check, marker_in};

#[test]
fn every_marker_is_flagged_as_a_word_of_its_own() {
    for marker in MARKERS {
        assert_eq!(marker_in(&format!("// {marker}: later")), Some(marker));
        assert_eq!(marker_in(&format!("x = 1 # {marker}")), Some(marker));
        assert_eq!(marker_in(&format!("/* ({marker}) */")), Some(marker));
    }
}

#[test]
fn a_marker_inside_a_longer_word_is_not_one() {
    let [_, _, xxx, hack] = MARKERS;
    assert_eq!(marker_in(&format!("color: #{xxx}{xxx};")), None);
    assert_eq!(marker_in(&format!("{hack}ING.md")), None);
    assert_eq!(marker_in(&format!("{hack}_COUNT")), None);
    assert_eq!(marker_in("Todo lists, in prose, are not markers"), None);
}

#[test]
fn a_flagged_line_names_its_file_and_line() {
    let text = format!("fn a() {{}}\n// {}: split this\n", MARKERS[1]);
    let [finding] = &check(Path::new("crates/core/src/a.rs"), &text)[..] else {
        panic!("expected one finding");
    };
    assert_eq!(finding.line, Some(2));
    assert_eq!(finding.rule, RULE);
    assert!(finding.message.contains(MARKERS[1]));
}

#[test]
fn an_allow_with_a_reason_excuses_the_line_below_it() {
    let text = format!(
        "// lint: allow({RULE}) quoting the vendor's own comment\n// {}\n",
        MARKERS[0]
    );
    assert!(check(Path::new("a.rs"), &text).is_empty());
    let text = format!("// lint: allow({RULE})\n// {}\n", MARKERS[0]);
    assert_eq!(check(Path::new("a.rs"), &text).len(), 1);
}
