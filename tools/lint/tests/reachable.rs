//! Every `.rs` under a crate's `tests/suite/` is a module the suite's
//! `main.rs` reaches: an undeclared file is flagged and named with the module
//! that should declare it, `#[path]` and `x/mod.rs` are followed, data trees
//! stay out, and an allow excuses an `include!`d file.

use std::path::{Path, PathBuf};

use scorsese_lint::rules::reachable::{RULE, check};

const TEST: &str = "//! A test.\n\n#[test]\nfn a() {}\n";

/// The paths `check` flags in a tree of `(path, text)`.
fn flagged(tree: &[(&str, &str)]) -> Vec<PathBuf> {
    let files: Vec<(&Path, &str)> = tree
        .iter()
        .map(|(path, text)| (Path::new(*path), *text))
        .collect();
    check(&files).into_iter().map(|f| f.path).collect()
}

#[test]
fn a_file_no_module_declares_is_flagged_and_names_its_declarer() {
    let tree = [
        ("crates/mcp/tests/suite/main.rs", "mod protocol;\n"),
        ("crates/mcp/tests/suite/protocol/mod.rs", "mod tools;\n"),
        ("crates/mcp/tests/suite/protocol/tools.rs", TEST),
        ("crates/mcp/tests/suite/protocol/scripting.rs", TEST),
    ];
    let files: Vec<(&Path, &str)> = tree.iter().map(|(p, t)| (Path::new(*p), *t)).collect();
    let [finding] = &check(&files)[..] else {
        panic!("expected one finding");
    };
    assert_eq!(finding.rule, RULE);
    assert_eq!(
        finding.path,
        Path::new("crates/mcp/tests/suite/protocol/scripting.rs")
    );
    assert!(
        finding.message.contains("`mod scripting;`")
            && finding
                .message
                .contains("crates/mcp/tests/suite/protocol/mod.rs"),
        "{}",
        finding.message
    );
}

#[test]
fn path_attributes_and_both_module_layouts_are_followed() {
    let tree = [
        (
            "crates/cli/tests/suite/main.rs",
            "mod help;\npub(crate) mod common;\n#[cfg(unix)] mod nested;\nmod r#type;\n",
        ),
        // A `#[path]` in a file that is not a `mod.rs` is relative to its own
        // directory, not to the module directory it would otherwise use.
        (
            "crates/cli/tests/suite/help.rs",
            "#[path = \"help/walk.rs\"]\nmod walk;\n",
        ),
        ("crates/cli/tests/suite/help/walk.rs", TEST),
        (
            "crates/cli/tests/suite/common/mod.rs",
            "#[path = \"../../../../../tools/scratch.rs\"]\nmod scratch;\n#[path = \"shared.rs\"] mod renamed;\n",
        ),
        ("crates/cli/tests/suite/common/shared.rs", TEST),
        ("tools/scratch.rs", TEST),
        // A module that is not a `mod.rs` declares its children in a
        // directory named after it.
        ("crates/cli/tests/suite/nested.rs", "mod deeper;\n"),
        ("crates/cli/tests/suite/nested/deeper.rs", TEST),
        ("crates/cli/tests/suite/type.rs", TEST),
        // Data, not modules.
        ("crates/core/tests/fixtures/make.rs", TEST),
        ("crates/compositor/tests/fonts/x.rs", TEST),
        ("crates/core/src/orphan.rs", TEST),
    ];
    assert_eq!(flagged(&tree), Vec::<PathBuf>::new());
}

#[test]
fn a_file_reached_from_another_crates_suite_is_reached() {
    let tree = [
        ("crates/zimmer/tests/suite/main.rs", "mod defaults;\n"),
        ("crates/zimmer/tests/suite/defaults.rs", TEST),
        ("crates/zimmer/tests/suite/common/written.rs", TEST),
        ("crates/providers/tests/suite/main.rs", "mod documented;\n"),
        (
            "crates/providers/tests/suite/documented/mod.rs",
            "#[path = \"../../../../zimmer/tests/suite/common/written.rs\"]\nmod written;\n",
        ),
    ];
    assert_eq!(flagged(&tree), Vec::<PathBuf>::new());
}

#[test]
fn comments_and_inline_modules_declare_nothing() {
    let tree = [
        (
            "crates/core/tests/suite/main.rs",
            "// mod commented;\n//! mod documented;\nmod inline {\n    fn a() {}\n}\n",
        ),
        ("crates/core/tests/suite/commented.rs", TEST),
        ("crates/core/tests/suite/documented.rs", TEST),
        ("crates/core/tests/suite/inline.rs", TEST),
    ];
    assert_eq!(flagged(&tree).len(), 3);
}

#[test]
fn a_suite_with_no_root_asks_for_its_main_rs() {
    let files = [(Path::new("crates/a/tests/suite/t.rs"), TEST)];
    let [finding] = &check(&files)[..] else {
        panic!("expected one finding");
    };
    assert!(
        finding.message.contains("crates/a/tests/suite/main.rs"),
        "{}",
        finding.message
    );
}

#[test]
fn the_app_panels_tree_is_walked_too() {
    let tree = [
        ("app/tests/panels/main.rs", "mod drawing;\n"),
        ("app/tests/panels/drawing.rs", TEST),
        ("app/tests/panels/watchdog/mod.rs", TEST),
    ];
    assert_eq!(
        flagged(&tree),
        [PathBuf::from("app/tests/panels/watchdog/mod.rs")]
    );
}

#[test]
fn an_allow_in_the_opening_comments_excuses_it() {
    let excused =
        "//! Spliced in.\n// lint: allow(unreachable-test) `include!`d by main.rs.\n\nfn a() {}\n";
    let bare = "// lint: allow(unreachable-test)\nfn a() {}\n";
    let tree = [
        (
            "crates/core/tests/suite/main.rs",
            "include!(\"spliced.rs\");\n",
        ),
        ("crates/core/tests/suite/spliced.rs", excused),
        ("crates/core/tests/suite/bare.rs", bare),
    ];
    assert_eq!(
        flagged(&tree),
        [PathBuf::from("crates/core/tests/suite/bare.rs")]
    );
}
