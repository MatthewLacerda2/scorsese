//! The policy reaches every crate, and the lint itself stays dependency-free.
//!
//! `policy.rs` holds the three `[workspace.lints]` tables equal; a table is
//! only the policy for the crates that take it, so this holds every member of
//! the root workspace to `[lints] workspace = true` (#1229). And `deny.toml`
//! notes that this crate has no dependencies, which its `Cargo.toml` promises
//! in a comment: the lockfile is where that is true or not.

use std::fs;
use std::path::{Path, PathBuf};

/// The repository root, reached from this crate's manifest directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("tools/lint sits two directories below the repository root")
        .to_path_buf()
}

/// The quoted paths in the root manifest's `members = [ … ]`.
fn members(manifest: &str) -> Vec<String> {
    let start = manifest
        .find("members = [")
        .expect("the root manifest lists its members");
    let list = &manifest[start..];
    let list = &list[..list.find(']').expect("the members list closes")];
    list.lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split('"').nth(1))
        .map(str::to_string)
        .collect()
}

/// Whether a manifest takes the workspace's lints.
fn takes_the_policy(manifest: &str) -> bool {
    let mut lines = manifest.lines().map(str::trim);
    lines.any(|line| line == "[lints]") && lines.next() == Some("workspace = true")
}

#[test]
fn every_member_takes_the_workspace_lints() {
    let root = repo_root();
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("root manifest");
    let members = members(&manifest);
    assert!(
        members.len() > 5,
        "found only {members:?} — the members list parser has stopped reading it"
    );
    for member in members {
        let path = root.join(&member).join("Cargo.toml");
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(
            takes_the_policy(&text),
            "{member} does not set `[lints] workspace = true`, so the workspace's \
             lint policy does not apply to it"
        );
    }
}

#[test]
fn a_manifest_that_does_not_take_the_policy_is_told_apart() {
    assert!(takes_the_policy(
        "[package]\nname = \"a\"\n\n[lints]\nworkspace = true\n"
    ));
    assert!(!takes_the_policy("[package]\nname = \"a\"\n"));
    assert!(!takes_the_policy(
        "[lints]\n[lints.clippy]\ntodo = \"deny\"\n"
    ));
}

#[test]
fn the_lint_has_no_dependencies() {
    let lock = fs::read_to_string(repo_root().join("tools/lint/Cargo.lock")).expect("lockfile");
    let packages: Vec<&str> = lock
        .lines()
        .filter_map(|line| line.strip_prefix("name = "))
        .collect();
    assert_eq!(
        packages,
        ["\"scorsese-lint\""],
        "tools/lint gained a dependency. A gate CI runs on every push should cost \
         seconds to compile, not a dependency tree of its own; see its Cargo.toml"
    );
}
