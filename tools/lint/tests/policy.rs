//! Every separate workspace root must state the same lint policy.
//!
//! Two directories are their own workspace roots — `tools/lint`, so the size
//! gate builds without ffmpeg, and `app`, so a graphics stack never reaches
//! `cargo test --workspace`. Each buys that isolation and pays every
//! inheritance for it, the lint policy included, so each restates the policy in
//! its own `Cargo.toml` and `clippy.toml` rather than inheriting one.
//!
//! Duplication that nothing checks is the failure mode the workspace table
//! existed to prevent, so these tests check it. They compare *settings* and not
//! bytes: the two copies carry different comments on purpose, each explaining
//! itself where it sits.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The repository root, reached from this crate's manifest directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("tools/lint sits two directories below the repository root")
        .to_path_buf()
}

/// Every `key = value` pair in `file`, keyed by `section.key`.
///
/// Enough TOML for two hand-written manifests and no more: comments and blank
/// lines are dropped, `[header]` opens a section, and a value keeps whatever
/// quoting it was written with so `"warn"` cannot silently equal `warn`.
fn settings(file: &Path) -> BTreeMap<String, String> {
    let text =
        std::fs::read_to_string(file).unwrap_or_else(|e| panic!("reading {}: {e}", file.display()));
    let mut section = String::new();
    let mut found = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(header) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = header.to_string();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = if section.is_empty() {
            key.trim().to_string()
        } else {
            format!("{section}.{}", key.trim())
        };
        found.insert(key, value.trim().to_string());
    }
    found
}

/// The `[workspace.lints.*]` entries of a manifest, with the prefix dropped.
fn lints(manifest: &Path) -> BTreeMap<String, String> {
    settings(manifest)
        .into_iter()
        .filter_map(|(key, value)| {
            key.strip_prefix("workspace.lints.")
                .map(|k| (k.to_string(), value))
        })
        .collect()
}

/// Every directory that is its own workspace root, and so inherits nothing.
///
/// Adding a third is one line here, and that is the point: the check is over
/// the *set* of separate roots rather than over one favoured one.
const SEPARATE_ROOTS: &[&str] = &["tools/lint", "app"];

#[test]
fn the_lint_tables_say_the_same_thing_at_every_root() {
    let root = repo_root();
    let workspace = lints(&root.join("Cargo.toml"));
    assert!(
        !workspace.is_empty(),
        "no [workspace.lints.*] found at the repo root — this test would pass vacuously"
    );
    for separate in SEPARATE_ROOTS {
        assert_eq!(
            workspace,
            lints(&root.join(separate).join("Cargo.toml")),
            "the lint policy at the repo root and in {separate} have drifted. \
             They are separate workspaces, so neither inherits the other: add \
             the lint to both tables, or remove it from both."
        );
    }
}

#[test]
fn the_clippy_configuration_says_the_same_thing_at_every_root() {
    let root = repo_root();
    let workspace = settings(&root.join("clippy.toml"));
    assert!(
        !workspace.is_empty(),
        "no settings found in the repo root clippy.toml — this test would pass vacuously"
    );
    for separate in SEPARATE_ROOTS {
        assert_eq!(
            workspace,
            settings(&root.join(separate).join("clippy.toml")),
            "clippy.toml at the repo root and in {separate} have drifted. clippy \
             reads it from the workspace root, and {separate} is its own \
             workspace, so a setting in one has no effect on the other."
        );
    }
}

#[test]
fn every_separate_root_takes_the_policy_rather_than_only_stating_it() {
    let root = repo_root();
    for separate in SEPARATE_ROOTS {
        let settings = settings(&root.join(separate).join("Cargo.toml"));
        assert_eq!(
            settings.get("lints.workspace").map(String::as_str),
            Some("true"),
            "{separate} declares [workspace.lints] but its package does not \
             take them with `[lints] workspace = true`, so they apply to nothing"
        );
    }
}
