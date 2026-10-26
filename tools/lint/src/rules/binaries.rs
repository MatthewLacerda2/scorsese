//! Each crate links one integration-test binary (#1128).
//!
//! Cargo builds every `tests/*.rs` and every `tests/*/main.rs` as a test
//! binary of its own, and each one links the crate and everything under it
//! again. So a crate's integration tests are modules of one binary,
//! `crates/<crate>/tests/suite/main.rs`, and a file Cargo would link alone is
//! flagged: it belongs in the suite as a module. Helpers, fixtures and the
//! suite's own modules are not binaries and are not read.
//!
//! An exception is a `lint: allow(test-binaries) <why>` comment in the run of
//! comment lines the file opens with.

use std::path::{Component, Path};

use super::Finding;
use super::allow::{grants, is_comment};

/// The rule's name, as an `allow` names it.
pub const RULE: &str = "test-binaries";

/// The directory under `tests/` that holds a crate's one binary.
pub const SUITE: &str = "suite";

/// Whether Cargo would build `path` as an integration-test binary of a crate
/// under `crates/` other than the suite.
fn is_binary(path: &Path) -> bool {
    let parts: Vec<&str> = path
        .components()
        .filter_map(|part| match part {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect();
    match parts[..] {
        ["crates", _, "tests", file] => file.ends_with(".rs"),
        ["crates", _, "tests", dir, "main.rs"] => dir != SUITE,
        _ => false,
    }
}

/// A finding when `path` would be a test binary of its own, unless the
/// comments it opens with excuse it.
pub fn check(path: &Path, text: &str) -> Vec<Finding> {
    if !is_binary(path) {
        return Vec::new();
    }
    let excused = text
        .lines()
        .take_while(|line| is_comment(line) || line.trim().is_empty())
        .any(|line| grants(line, RULE));
    if excused {
        return Vec::new();
    }
    vec![Finding {
        path: path.to_path_buf(),
        line: None,
        rule: RULE,
        message: format!(
            "a test binary of its own, linked apart from the crate's one: make it a module of \
             `tests/{SUITE}/` and declare it in `tests/{SUITE}/main.rs`"
        ),
    }]
}
