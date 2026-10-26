//! Edges a manifest declares without a plain key: a dotted key, a rename,
//! a commented header and a `[dependencies.foo]` table.

use std::path::Path;

use scorsese_lint::rules::Finding;
use scorsese_lint::rules::boundary::edges::{EDGE, check};
use scorsese_lint::rules::boundary::manifest::parse;

fn findings(text: &str) -> Vec<Finding> {
    check(Path::new("crates/x/Cargo.toml"), text, &parse(text))
}

#[test]
fn a_dotted_key_is_an_edge_like_any_other() {
    let text =
        "[package]\nname = \"scorsese-core\"\n[dependencies]\nscorsese-render.workspace = true\n";
    let [finding] = &findings(text)[..] else {
        panic!("expected one finding: {:?}", findings(text));
    };
    assert_eq!((finding.rule, finding.line), (EDGE, Some(4)));
    let text = "[package]\nname = \"scorsese-compositor\"\n[dependencies]\nscorsese-core.workspace = true\nstrsim.workspace = true\n";
    assert!(findings(text).is_empty(), "{:?}", findings(text));
}

#[test]
fn a_dotted_rename_is_read_as_a_string_in_either_quotes() {
    let core = |deps: &str| {
        findings(&format!(
            "[package]\nname = \"scorsese-core\"\n[dependencies]\n{deps}"
        ))
    };
    for forbidden in [
        "alias.package = 'scorsese-render'\n",
        "alias.\"package\" = \"scorsese-render\"\n",
        "'scorsese-render'.workspace = true\n",
    ] {
        let found = core(forbidden);
        assert_eq!(found.len(), 1, "{forbidden}: {found:?}");
        assert_eq!(found[0].rule, EDGE);
    }
    let text = "[package]\nname = \"scorsese-compositor\"\n[dependencies]\ncore.package = \"scorsese-core\" # renamed\ncore.path = \"../core\"\n";
    assert!(findings(text).is_empty(), "{:?}", findings(text));
}

#[test]
fn a_dependency_table_is_read_under_a_commented_header() {
    let text =
        "[package]\nname = \"scorsese-core\"\n[dependencies] # runtime\nscorsese-render = \"1\"\n";
    let [finding] = &findings(text)[..] else {
        panic!("expected one finding: {:?}", findings(text));
    };
    assert_eq!((finding.rule, finding.line), (EDGE, Some(4)));
    let text = "[package]\nname = \"scorsese-compositor\"\n[dependencies] # runtime\nscorsese-core = \"1\"\n";
    assert!(findings(text).is_empty(), "{:?}", findings(text));
}

#[test]
fn a_named_dependency_table_is_an_edge_under_its_rename() {
    let text = "[package]\nname = \"scorsese-core\"\n[dependencies.alias]\npackage = \"scorsese-render\"\npath = \"../render\"\n";
    let [finding] = &findings(text)[..] else {
        panic!("expected one finding: {:?}", findings(text));
    };
    assert_eq!((finding.rule, finding.line), (EDGE, Some(3)));
    let text = "[package]\nname = \"scorsese-compositor\"\n[dependencies.alias]\npackage = \"scorsese-core\"\n";
    assert!(findings(text).is_empty(), "{:?}", findings(text));
}
