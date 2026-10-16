//! Crate edges, leaves and owners, read from manifests.

use std::path::Path;

use scorsese_lint::rules::Finding;
use scorsese_lint::rules::boundary::edges::{ALLOWED, EDGE, OWNER, check};
use scorsese_lint::rules::boundary::manifest::parse;

fn findings(text: &str) -> Vec<Finding> {
    check(Path::new("crates/x/Cargo.toml"), text, &parse(text))
}

#[test]
fn every_allowed_edge_passes() {
    for (name, deps) in ALLOWED {
        let mut text = format!("[package]\nname = \"{name}\"\n[dependencies]\n");
        for dep in *deps {
            text.push_str(&format!("{dep} = {{ path = \"..\" }}\n"));
        }
        assert!(findings(&text).is_empty(), "{name}: {:?}", findings(&text));
    }
}

#[test]
fn an_edge_the_table_does_not_hold_is_flagged_at_its_line() {
    let text = "[package]\nname = \"scorsese-zimmer\"\n[dependencies]\nscorsese-core = { path = \"../core\" }\n";
    let [finding] = &findings(text)[..] else {
        panic!("expected one finding: {:?}", findings(text));
    };
    assert_eq!((finding.rule, finding.line), (EDGE, Some(4)));
    assert!(
        finding.message.contains("scorsese-core"),
        "{}",
        finding.message
    );
}

#[test]
fn a_dev_dependency_is_an_edge_and_a_self_dependency_is_not() {
    let text = "[package]\nname = \"scorsese-core\"\n[dev-dependencies]\nscorsese-core = { path = \".\" }\nscorsese-cli = { path = \"../cli\" }\n";
    let found = findings(text);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, Some(5));
}

#[test]
fn a_crate_of_ours_missing_from_the_table_is_flagged_and_a_stranger_is_not() {
    let found = findings("[package]\nname = \"scorsese-new\"\n");
    assert_eq!(found.len(), 1);
    assert!(found[0].message.contains("not in the crate-edge table"));
    assert!(
        findings("[package]\nname = \"someone-else\"\n[dependencies]\nscorsese-cli = \"1\"\n")
            .is_empty()
    );
}

#[test]
fn a_leaf_may_depend_on_nothing_at_all() {
    let text = "[package]\nname = \"scorsese-guide\"\n[dependencies]\nserde = \"1\"\n";
    let [finding] = &findings(text)[..] else {
        panic!("expected one finding");
    };
    assert!(finding.message.contains("leaf"), "{}", finding.message);
}

#[test]
fn an_http_or_postgres_client_belongs_to_its_one_owner() {
    let held = |name: &str, dep: &str| {
        findings(&format!(
            "[package]\nname = \"{name}\"\n[dependencies]\n{dep} = \"1\"\n"
        ))
    };
    assert!(held("scorsese-providers", "ureq").is_empty());
    assert!(held("scorsese-server", "sqlx").is_empty());
    for (name, dep) in [
        ("scorsese-render", "ureq"),
        ("scorsese-mcp", "reqwest"),
        ("scorsese-cli", "sqlx"),
    ] {
        let found = held(name, dep);
        assert_eq!(found.len(), 1, "{name} on {dep}");
        assert_eq!(found[0].rule, OWNER);
    }
}

#[test]
fn an_allow_excuses_the_line_below_it_and_not_the_one_above() {
    let text = format!(
        "[package]\nname = \"scorsese-guide\"\n[dependencies]\nserde = \"1\"\n# lint: allow({EDGE}) a probe, filed as #1\nscorsese-cli = \"1\"\n"
    );
    let found = findings(&text);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, Some(4));
    let text = format!(
        "[package]\nname = \"scorsese-guide\"\n[dependencies]\n# lint: allow({EDGE}) generated, filed as #1\nserde = \"1\"\nscorsese-cli = \"1\"\n"
    );
    let found = findings(&text);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, Some(6));
}

#[test]
fn an_allow_with_a_reason_excuses_one_line() {
    let text = format!(
        "[package]\nname = \"scorsese-render\"\n[dependencies]\n# lint: allow({OWNER}) a probe, filed as #1\nureq = \"3\"\n# lint: allow({EDGE})\nscorsese-cli = \"1\"\n"
    );
    let found = findings(&text);
    assert_eq!(
        found.len(),
        1,
        "a reasonless allow excuses nothing: {found:?}"
    );
    assert_eq!(found[0].rule, EDGE);
}
