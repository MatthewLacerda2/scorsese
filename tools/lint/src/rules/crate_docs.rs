//! Each crate's module doc names its public modules (each `lib.rs`, *What
//! this publishes*).
//!
//! A crate doc is where a reader, or an agent deciding where a change goes,
//! learns what the crate offers; a `pub mod` it never mentions is a part of
//! the crate nobody is told about. render's and providers' had drifted that
//! way before #1219 fixed them by hand; this keeps them fixed.
//!
//! Named means written as code in the doc: `` `name` ``, `[name]`,
//! `` `mod@name` `` or `` `name::…` ``. Only `lib.rs` directly in a `src`
//! directory is read, and only its unindented `pub mod name;` lines.

use std::path::Path;

use super::Finding;
use super::allow::allowed;

/// The rule's name, as an `allow` names it.
pub const RULE: &str = "crate-doc";

/// Whether `path` is a crate root this rule reads.
fn is_crate_root(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == "lib.rs")
        && path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|dir| dir == "src")
}

/// Whether the crate doc `doc` names the module `name`.
pub fn names(doc: &str, name: &str) -> bool {
    [
        format!("`{name}`"),
        format!("[{name}]"),
        format!("`mod@{name}`"),
        format!("`{name}::"),
    ]
    .iter()
    .any(|form| doc.contains(form.as_str()))
}

/// Every root `pub mod` of a crate root that its module doc does not name.
pub fn check(path: &Path, text: &str) -> Vec<Finding> {
    if !is_crate_root(path) {
        return Vec::new();
    }
    let lines: Vec<&str> = text.lines().collect();
    let doc: String = lines
        .iter()
        .filter_map(|line| line.strip_prefix("//!"))
        .collect::<Vec<_>>()
        .join("\n");
    lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            let name = line.strip_prefix("pub mod ")?.strip_suffix(';')?;
            Some((index, name))
        })
        .filter(|(index, name)| !names(&doc, name) && !allowed(&lines, *index, RULE))
        .map(|(index, name)| Finding {
            path: path.to_path_buf(),
            line: Some(index + 1),
            rule: RULE,
            message: format!(
                "`pub mod {name}` is not named in the crate doc: say what it publishes"
            ),
        })
        .collect()
}
