//! How declarations are read, as rustc resolves them: `#[path]` (also in
//! `cfg_attr`), both module layouts, another crate's suite, trailing and
//! whole-line comments, and inline modules.

use std::path::PathBuf;

use super::{TEST, flagged};

#[test]
fn path_attributes_and_both_module_layouts_are_followed() {
    let tree = [
        (
            "crates/cli/tests/suite/main.rs",
            "mod help;\npub(crate) mod common;\nmod the_rest;\n#[cfg(unix)] mod nested;\nmod r#type;\n",
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
            "#[path = \"../../../../../tools/scratch.rs\"]\nmod scratch;\n#[path = \"shared.rs\"]\n// Renamed.\nmod renamed;\n",
        ),
        ("crates/cli/tests/suite/common/shared.rs", TEST),
        ("tools/scratch.rs", TEST),
        // A module that is not a `mod.rs` declares its children in a
        // directory named after it.
        ("crates/cli/tests/suite/nested.rs", "mod deeper;\n"),
        ("crates/cli/tests/suite/nested/deeper.rs", TEST),
        ("crates/cli/tests/suite/type.rs", TEST),
        ("crates/cli/tests/suite/the_rest.rs", TEST),
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
fn trailing_comments_cfg_attr_paths_and_inline_modules_are_read_as_rustc_does() {
    let tree = [
        (
            "crates/core/tests/suite/main.rs",
            "mod render; // slow\n#[cfg_attr(all(unix, test), path = \"unix_impl.rs\")]\nmod platform;\nmod y {\n    mod x;\n}\n",
        ),
        ("crates/core/tests/suite/render.rs", TEST),
        ("crates/core/tests/suite/unix_impl.rs", TEST),
        ("crates/core/tests/suite/x.rs", TEST),
    ];
    assert_eq!(
        flagged(&tree),
        [PathBuf::from("crates/core/tests/suite/x.rs")]
    );
}
