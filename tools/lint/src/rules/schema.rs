//! A test document at this build's version says so by name (#1279).
//!
//! A Rust test that writes `"schema_version": 49` into a document means *the
//! version this build reads*, and is right only until the next bump. Then it
//! either fails, or quietly starts testing the migration instead of what it was
//! written for. Interpolating `SCHEMA_VERSION` keeps meaning what it says; the
//! bump stops touching these files at all.
//!
//! Only a literal **equal to the current version** is flagged: a document at
//! an older one is a migration's input, written at that version on purpose.
//! The current version is read from where it is defined, [`DEFINED_IN`]; a tree
//! without that file has no version to hold anything to, and the rule is off.
//! Rust only — the web and the docs write JSON that cannot interpolate.

use std::path::Path;

use super::Finding;
use super::allow::{allowed, is_comment};

/// The rule's name, as an `allow` names it.
pub const RULE: &str = "schema-literal";

/// The file that defines `SCHEMA_VERSION`, relative to the scan root.
pub const DEFINED_IN: &str = "crates/core/src/project/mod.rs";

/// The version `text` (the file at [`DEFINED_IN`]) defines, if it defines one.
pub fn defined_version(text: &str) -> Option<u32> {
    text.lines().find_map(|line| {
        let rest = line
            .trim()
            .strip_prefix("pub const SCHEMA_VERSION: u32 = ")?;
        rest.strip_suffix(';')?.trim().parse().ok()
    })
}

/// Every `"schema_version": <n>` on `line`, as the number written.
fn literals(line: &str) -> impl Iterator<Item = u32> + '_ {
    line.match_indices("\"schema_version\"")
        .filter_map(|(at, key)| {
            let rest = line[at + key.len()..].trim_start().strip_prefix(':')?;
            let rest = rest.trim_start();
            let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
            rest[..digits].parse().ok()
        })
}

/// Every line of the Rust file `text` writing `current` as a literal schema
/// version, that no allow excuses.
pub fn check(path: &Path, text: &str, current: u32) -> Vec<Finding> {
    if path.extension().is_none_or(|ext| ext != "rs") {
        return Vec::new();
    }
    let lines: Vec<&str> = text.lines().collect();
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| !is_comment(line))
        .filter(|(_, line)| literals(line).any(|written| written == current))
        .filter(|(index, _)| !allowed(&lines, *index, RULE))
        .map(|(index, _)| Finding {
            path: path.to_path_buf(),
            line: Some(index + 1),
            rule: RULE,
            message: format!(
                "`\"schema_version\": {current}` is this build's version: interpolate \
                 `SCHEMA_VERSION` so the next bump does not have to find this line"
            ),
        })
        .collect()
}
