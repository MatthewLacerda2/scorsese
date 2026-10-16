//! A Rust test document at this build's schema version names the constant
//! rather than writing the number; a web one never writes a number, and the
//! web's constant is held to the Rust one.

use std::path::Path;

use scorsese_lint::rules::schema::{
    RULE, WEB_DEFINED_IN, check, check_web_definition, defined_version, web_defined_version,
};

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
fn a_web_document_writing_any_version_is_flagged() {
    let text = "const doc = {\n  schema_version: 1,\n  name: \"t\",\n};\n\
                const json = { \"schema_version\": 49 };\n";
    assert_eq!(found("web/src/editor/a.test.ts", text), [2, 5]);
    assert_eq!(found("web/src/b.test.tsx", "{ schema_version: 51 }\n"), [1]);
}

#[test]
fn a_web_import_a_type_or_another_language_is_not() {
    let text = "const doc = { schema_version: SCHEMA_VERSION };\n\
                interface P { schema_version: number; }\n\
                // schema_version: 1 in prose\n";
    assert!(found("web/src/a.test.ts", text).is_empty());
    assert!(found("docs/a.md", "{ \"schema_version\": 49 }\n").is_empty());
    assert!(found("a.py", "{ \"schema_version\": 49 }\n").is_empty());
}

#[test]
fn the_web_constant_is_held_to_the_rust_one() {
    let text = "/** The version. */\nexport const SCHEMA_VERSION = 49;\n";
    assert_eq!(web_defined_version(text), Some(49));
    assert_eq!(check_web_definition(text, 49), None);
    let stale = check_web_definition(text, 50).expect("49 is not 50");
    assert_eq!(
        (stale.rule, stale.path.to_str()),
        (RULE, Some(WEB_DEFINED_IN))
    );
    assert!(check_web_definition("export const OTHER = 49;\n", 49).is_some());
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
