//! The lint policy here must match the repo root's.
//!
//! This crate is its own workspace root (see the comment atop its `Cargo.toml`),
//! which buys a gate that builds without ffmpeg and stays out of
//! `cargo test --workspace` — and costs it every inheritance, the lint policy
//! included. The policy is therefore restated in `tools/lint/Cargo.toml` and
//! `tools/lint/clippy.toml` rather than inherited.
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
    let text = std::fs::read_to_string(file)
        .unwrap_or_else(|e| panic!("reading {}: {e}", file.display()));
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

#[test]
fn the_lint_tables_say_the_same_thing_at_both_roots() {
    let root = repo_root();
    let workspace = lints(&root.join("Cargo.toml"));
    let tool = lints(&root.join("tools/lint/Cargo.toml"));

    assert!(
        !workspace.is_empty(),
        "no [workspace.lints.*] found at the repo root — this test would pass vacuously"
    );
    assert_eq!(
        workspace, tool,
        "the lint policy at the repo root and in tools/lint have drifted. \
         They are separate workspaces, so neither inherits the other: add the \
         lint to both tables, or remove it from both."
    );
}

#[test]
fn the_clippy_configuration_says_the_same_thing_at_both_roots() {
    let root = repo_root();
    let workspace = settings(&root.join("clippy.toml"));
    let tool = settings(&root.join("tools/lint/clippy.toml"));

    assert!(
        !workspace.is_empty(),
        "no settings found in the repo root clippy.toml — this test would pass vacuously"
    );
    assert_eq!(
        workspace, tool,
        "clippy.toml at the repo root and in tools/lint have drifted. clippy \
         reads it from the workspace root, and tools/lint is its own workspace, \
         so a setting in one has no effect on the other."
    );
}

#[test]
fn this_crate_takes_the_policy_rather_than_only_stating_it() {
    let manifest = repo_root().join("tools/lint/Cargo.toml");
    let settings = settings(&manifest);

    assert_eq!(
        settings.get("lints.workspace").map(String::as_str),
        Some("true"),
        "tools/lint declares [workspace.lints] but its package does not take \
         them with `[lints] workspace = true`, so they apply to nothing"
    );
}
