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
//!
//! **The web front-end holds itself the same way, harder** (#1215). Its copy
//! of the constant, in [`WEB_DEFINED_IN`], must equal the Rust one, and a
//! `.ts` or `.tsx` document writes no version number at all: the web has no
//! migration tests, so a literal there is only ever stale — four test
//! documents still said `1` at version 51. The docs and `.json` fixtures write
//! JSON that cannot interpolate, and are not read.

use std::path::Path;

use super::Finding;
use super::allow::{allowed, is_comment};

/// The rule's name, as an `allow` names it.
pub const RULE: &str = "schema-literal";

/// The file that defines `SCHEMA_VERSION`, relative to the scan root.
pub const DEFINED_IN: &str = "crates/core/src/project/mod.rs";

/// The web front-end's copy of `SCHEMA_VERSION`, relative to the scan root.
pub const WEB_DEFINED_IN: &str = "web/src/api/document.ts";

/// The version `text` (the file at [`DEFINED_IN`]) defines, if it defines one.
pub fn defined_version(text: &str) -> Option<u32> {
    text.lines().find_map(|line| {
        let rest = line
            .trim()
            .strip_prefix("pub const SCHEMA_VERSION: u32 = ")?;
        rest.strip_suffix(';')?.trim().parse().ok()
    })
}

/// The version the web's `text` (the file at [`WEB_DEFINED_IN`]) defines.
pub fn web_defined_version(text: &str) -> Option<u32> {
    text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("export const SCHEMA_VERSION = ")?;
        rest.strip_suffix(';')?.trim().parse().ok()
    })
}

/// A finding on the web's definition, `text`, unless it says `current`.
pub fn check_web_definition(text: &str, current: u32) -> Option<Finding> {
    let written = web_defined_version(text);
    (written != Some(current)).then(|| Finding {
        path: WEB_DEFINED_IN.into(),
        line: None,
        rule: RULE,
        message: format!(
            "the web's `export const SCHEMA_VERSION = {current};` must say the version \
             `{DEFINED_IN}` defines, {current}; it says {}",
            written.map_or("nothing".to_owned(), |v| v.to_string())
        ),
    })
}

/// Every `<key>: <n>` on `line`, as the number written.
fn literals<'a>(line: &'a str, key: &'a str) -> impl Iterator<Item = u32> + 'a {
    line.match_indices(key).filter_map(move |(at, key)| {
        let rest = line[at + key.len()..].trim_start_matches('"').trim_start();
        let rest = rest.strip_prefix(':')?;
        let rest = rest.trim_start();
        let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        rest[..digits].parse().ok()
    })
}

/// Every line of the code file `text` writing a schema version it should
/// not, that no allow excuses: in Rust, `"schema_version": <current>`; in the
/// web's TypeScript, `schema_version: <any number>`.
pub fn check(path: &Path, text: &str, current: u32) -> Vec<Finding> {
    let web = match path.extension().and_then(|ext| ext.to_str()) {
        Some("rs") => false,
        Some("ts" | "tsx") => true,
        _ => return Vec::new(),
    };
    let key = if web {
        "schema_version"
    } else {
        "\"schema_version\""
    };
    let lines: Vec<&str> = text.lines().collect();
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| !is_comment(line))
        .filter(|(_, line)| literals(line, key).any(|written| web || written == current))
        .filter(|(index, _)| !allowed(&lines, *index, RULE))
        .map(|(index, _)| Finding {
            path: path.to_path_buf(),
            line: Some(index + 1),
            rule: RULE,
            message: if web {
                "a web document writes no schema version number: import `SCHEMA_VERSION` \
                 from `@/api`, which is held to the Rust one"
                    .to_owned()
            } else {
                format!(
                    "`\"schema_version\": {current}` is this build's version: interpolate \
                     `SCHEMA_VERSION` so the next bump does not have to find this line"
                )
            },
        })
        .collect()
}
