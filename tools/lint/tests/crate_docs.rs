//! Each crate's module doc names its public modules.

use std::path::Path;

use scorsese_lint::rules::crate_docs::{RULE, check, names};

#[test]
fn every_way_of_writing_a_module_as_code_names_it() {
    for doc in ["`pool`", "[pool]", "[`mod@pool`]", "`pool::import`"] {
        assert!(names(doc, "pool"), "{doc}");
    }
    assert!(!names("the pool of media", "pool"));
    assert!(!names("`pools`", "pool"));
}

#[test]
fn a_public_module_the_doc_never_names_is_flagged() {
    let text = "//! [`mod@pool`] brings media in.\n\npub mod pool;\npub mod probe;\nmod scratch;\n";
    let [finding] = &check(Path::new("crates/core/src/lib.rs"), text)[..] else {
        panic!("expected one finding");
    };
    assert_eq!(finding.line, Some(4));
    assert!(finding.message.contains("probe"), "{}", finding.message);
}

#[test]
fn only_a_crate_root_is_read() {
    let text = "//! Nothing named.\npub mod hidden;\n";
    assert_eq!(check(Path::new("crates/core/src/lib.rs"), text).len(), 1);
    assert!(check(Path::new("crates/core/src/pool/mod.rs"), text).is_empty());
    assert!(check(Path::new("crates/core/tests/lib.rs"), text).is_empty());
}

#[test]
fn an_allow_with_a_reason_excuses_a_module() {
    let text =
        format!("//! Nothing named.\n// lint: allow({RULE}) re-exported whole\npub mod hidden;\n");
    assert!(check(Path::new("x/src/lib.rs"), &text).is_empty());
}
