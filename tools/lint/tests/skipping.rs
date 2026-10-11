//! Which directories the gate never walks. A skip is the one way the gate can
//! stop gating without failing, so each one is pinned to *where* it applies,
//! not only to its name: until #1131 a folder named `cache` anywhere was
//! skipped, and a source module of that name went unmeasured over the cap.

use std::path::Path;

use scorsese_lint::classify::{Kind, classify, is_skipped_dir};

fn kind_of(path: &str) -> Option<Kind> {
    classify(Path::new(path))
}

fn skipped(dir: &str) -> bool {
    is_skipped_dir(Path::new(dir))
}

#[test]
fn build_output_and_hidden_directories_are_not_measured() {
    for path in [
        "target/debug/build/scorsese-core-1/out/generated.rs",
        "app/target/debug/build/x/out/table.rs",
        "tools/lint/target/debug/build/x/out/table.rs",
        ".git/hooks/thing.rs",
        ".claude/worktrees/agent-1/crates/core/src/asset.rs",
        "some.scor/generated/x.rs",
        "some.scor/cache/x.rs",
        "projects/demo.scor/cache/pages/x.ts",
        "node_modules/whatever/x.rs",
        "web/node_modules/react/index.d.ts",
    ] {
        assert_eq!(kind_of(path), None, "{path}");
    }
}

#[test]
fn a_source_module_named_like_a_skipped_folder_is_measured() {
    assert_eq!(kind_of("crates/x/src/cache/y.rs"), Some(Kind::Source));
    assert_eq!(
        kind_of("crates/render/src/page/cache/mod.rs"),
        Some(Kind::Source)
    );
    assert_eq!(kind_of("crates/x/src/generated/y.rs"), Some(Kind::Source));
    assert_eq!(kind_of("crates/x/src/target/y.rs"), Some(Kind::Source));
    assert_eq!(kind_of("crates/x/tests/target/y.rs"), Some(Kind::Test));
    assert_eq!(kind_of("web/src/cache/store.ts"), Some(Kind::Source));
}

#[test]
fn cache_and_generated_are_skipped_only_directly_inside_a_scor_project() {
    assert!(skipped("demo.scor/cache"));
    assert!(skipped("examples/demo.scor/generated"));
    // One level down is the project's own content, not its scratch space —
    // and a folder that merely mentions `.scor` is not a project.
    assert!(!skipped("demo.scor/assets/cache"));
    assert!(!skipped("demo.scor.bak/x/cache"));
    assert!(!skipped("cache"));
    assert!(!skipped("generated"));
}

#[test]
fn target_is_skipped_only_where_a_workspace_root_would_put_it() {
    assert!(skipped("target"));
    assert!(skipped("app/target"));
    assert!(skipped("tools/lint/target"));
    for dir in [
        "crates/x/src/target",
        "crates/x/src/a/target",
        "crates/x/tests/target",
        "crates/x/benches/target",
        "crates/x/examples/target",
    ] {
        assert!(!skipped(dir), "{dir}");
    }
}

#[test]
fn the_skip_list_is_directories_only_not_files_that_look_like_them() {
    assert!(skipped(".github"));
    assert!(skipped("web/node_modules"));
    assert!(!skipped("tests"));
    assert!(!skipped("src"));
    assert!(!skipped(""));
    // A *file* named after a skipped directory is still measured — only the
    // directory components of a path are consulted.
    assert_eq!(kind_of("crates/core/src/target.rs"), Some(Kind::Source));
    assert_eq!(kind_of("demo.scor/cache.rs"), Some(Kind::Source));
}
