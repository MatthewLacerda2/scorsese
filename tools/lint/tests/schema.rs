//! A Rust test document at this build's schema version names the constant
//! rather than writing the number.

use std::path::Path;

use scorsese_lint::rules::schema::{RULE, check, defined_version};

/// The lines flagged in `text` at `path`, against version 49.
fn found(path: &str, text: &str) -> Vec<usize> {
    check(Path::new(path), text, 49)
        .into_iter()
        .map(|finding| {
            assert_eq!(finding.rule, RULE);
            finding.line.expect("a literal is on a line")
        })
        .collect()
}

#[test]
fn the_current_version_written_as_a_number_is_flagged() {
    let text = "const D: &str = r#\"{\n  \"schema_version\": 49,\n  \"name\": \"T\" }\"#;\n";
    assert_eq!(found("crates/core/tests/a.rs", text), [2]);
    let text = "json!({ \"schema_version\":49, \"name\": \"T\" })\n";
    assert_eq!(found("crates/server/tests/a.rs", text), [1]);
}

#[test]
fn an_older_version_an_interpolation_or_a_comment_is_not() {
    let text = "json!({ \"schema_version\": 33, \"trail\": [] })\n\
                format!(r#\"{{ \"schema_version\": {SCHEMA_VERSION} }}\"#)\n\
                // \"schema_version\": 49 in prose\n\
                json!({ \"schema_version\": 490 })\n";
    assert!(found("crates/core/src/migrate/tests.rs", text).is_empty());
}

#[test]
fn only_rust_is_read() {
    assert!(found("web/src/fixture.ts", "{ \"schema_version\": 49 }\n").is_empty());
}

#[test]
fn an_allow_with_a_reason_excuses_it() {
    let text =
        format!("// lint: allow({RULE}) pins the number on purpose\n\"schema_version\": 49\n");
    assert!(found("a.rs", &text).is_empty());
}

#[test]
fn the_version_is_read_from_its_definition() {
    let text = "/// The version.\npub const SCHEMA_VERSION: u32 = 49;\n";
    assert_eq!(defined_version(text), Some(49));
    assert_eq!(defined_version("pub const OTHER: u32 = 3;\n"), None);
}
