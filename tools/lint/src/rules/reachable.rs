//! Every `.rs` under a crate's `tests/suite/` is a module its `main.rs`
//! reaches (#1615).
//!
//! rustc compiles only the files a `mod` declaration names, so a test file no
//! module declares is never built and never run, while the size gate still
//! counts it as a test. So each suite root (`crates/<crate>/tests/suite/main.rs`,
//! and each `app/tests/<binary>/main.rs`) is walked through its `mod x;`
//! declarations, the way rustc resolves them: `x.rs` or `x/mod.rs` beside a
//! `main.rs` or `mod.rs`, under a directory named after any other file, and
//! `#[path = "…"]` relative to the declaring file's own directory. Every
//! `.rs` under a suite that no walk reaches is flagged. A file reached from
//! another crate's suite counts as reached: it is compiled there.
//!
//! The parse is by line and reads only unindented declarations: a `mod x;`
//! inside an inline `mod y { … }` is not followed, and the file it names is
//! flagged; none exists today.
//!
//! An exception, for a file brought in some other way (`include!`), is a
//! `lint: allow(unreachable-test) <why>` comment in the run of comment lines
//! the file opens with.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use super::Finding;
use super::allow::{grants, is_comment};

/// The rule's name, as an `allow` names it.
pub const RULE: &str = "unreachable-test";

/// The directories whose `main.rs` roots a module tree, and under which
/// every `.rs` must be reached from it.
fn suite_dir(path: &Path) -> Option<PathBuf> {
    let parts = names(path);
    match parts[..] {
        ["crates", krate, "tests", "suite", ..] => {
            Some(["crates", krate, "tests", "suite"].iter().collect())
        }
        ["app", "tests", binary, _, ..] => Some(["app", "tests", binary].iter().collect()),
        _ => None,
    }
}

/// The path's normal components, as text.
fn names(path: &Path) -> Vec<&str> {
    path.components()
        .filter_map(|part| match part {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect()
}

/// `path` with `.` and `..` resolved lexically, as rustc reads a `#[path]`.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

/// Whether a file roots its own module directory (`main.rs`, `lib.rs`,
/// `mod.rs`) rather than declaring children under a directory named after it.
fn is_mod_rs(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("main.rs" | "lib.rs" | "mod.rs")
    )
}

/// The `#[path = "…"]` value of one attribute's text, if it is one.
/// A `#[cfg_attr(<predicate>, path = "…")]` counts too: rustc compiles that
/// file on the platforms it names.
fn path_attribute(attribute: &str) -> Option<&str> {
    let attribute = match attribute.strip_prefix("cfg_attr(") {
        Some(inner) => inner.strip_suffix(')')?.rsplit(',').next()?.trim(),
        None => attribute,
    };
    let value = attribute
        .strip_prefix("path")?
        .trim_start()
        .strip_prefix('=')?
        .trim();
    value.strip_prefix('"')?.strip_suffix('"')
}

/// The outline modules `text` declares: each `mod x;` with its `#[path]`,
/// whether on its own line or on the attribute lines directly above. An
/// indented line is inside an inline module or an item and declares nothing
/// at the file's top level, so it is not read: a file only such a line names
/// is flagged rather than wrongly counted as reached.
fn declarations(text: &str) -> Vec<(String, Option<String>)> {
    let mut found = Vec::new();
    let mut pending: Option<String> = None;
    for line in text.lines() {
        let mut rest = line.trim();
        if is_comment(rest) || rest.is_empty() {
            continue;
        }
        if line.starts_with(char::is_whitespace) {
            pending = None;
            continue;
        }
        while let Some(after) = rest.strip_prefix("#[") {
            let Some(end) = after.find(']') else { break };
            if let Some(path) = path_attribute(after[..end].trim()) {
                pending = Some(path.to_owned());
            }
            rest = after[end + 1..].trim_start();
        }
        if rest.is_empty() {
            continue;
        }
        if let Some(name) = outline_module(rest) {
            found.push((name.trim_start_matches("r#").to_owned(), pending.take()));
        }
        pending = None;
    }
    found
}

/// The name in `[pub[(…)]] mod name;`, if the line is one.
fn outline_module(line: &str) -> Option<&str> {
    let mut rest = line.split("//").next().unwrap_or(line).trim_end();
    if let Some(after) = rest.strip_prefix("pub") {
        rest = after.trim_start();
        if let Some(scoped) = rest.strip_prefix('(') {
            rest = scoped.split_once(')')?.1.trim_start();
        }
    }
    let name = rest
        .strip_prefix("mod ")?
        .trim()
        .strip_suffix(';')?
        .trim_end();
    let bare = name.trim_start_matches("r#");
    (!bare.is_empty() && bare.chars().all(|c| c.is_alphanumeric() || c == '_')).then_some(name)
}

/// The files `file`'s declarations name that are in `files`.
fn children(file: &Path, text: &str, files: &BTreeMap<&Path, &str>) -> Vec<PathBuf> {
    let dir = file.parent().unwrap_or(Path::new(""));
    let own = if is_mod_rs(file) {
        dir.to_path_buf()
    } else {
        dir.join(file.file_stem().unwrap_or_default())
    };
    declarations(text)
        .into_iter()
        .filter_map(|(name, path)| {
            let candidates = match path {
                Some(path) => vec![normalize(&dir.join(path))],
                None => vec![
                    own.join(format!("{name}.rs")),
                    own.join(&name).join("mod.rs"),
                ],
            };
            candidates
                .into_iter()
                .find(|candidate| files.contains_key(candidate.as_path()))
        })
        .collect()
}

/// The module that should declare `file`, and the declaration it should hold.
fn declarer(file: &Path, files: &BTreeMap<&Path, &str>) -> (PathBuf, String) {
    let parent = file.parent().unwrap_or(Path::new(""));
    let (name, dir) = if file.file_name().is_some_and(|name| name == "mod.rs") {
        let name = parent.file_name().unwrap_or_default();
        (name, parent.parent().unwrap_or(Path::new("")))
    } else {
        (file.file_stem().unwrap_or_default(), parent)
    };
    let candidates = [
        dir.join("main.rs"),
        dir.join("mod.rs"),
        dir.with_extension("rs"),
    ];
    let module = candidates
        .iter()
        .find(|candidate| files.contains_key(candidate.as_path()))
        .unwrap_or(&candidates[usize::from(suite_dir(file).as_deref() != Some(dir))])
        .clone();
    (module, format!("mod {};", name.to_string_lossy()))
}

/// A finding for every `.rs` under a suite that no suite root reaches.
pub fn check(files: &[(&Path, &str)]) -> Vec<Finding> {
    let files: BTreeMap<&Path, &str> = files.iter().copied().collect();
    let mut queue: Vec<PathBuf> = files
        .keys()
        .filter(|path| {
            path.file_name().is_some_and(|name| name == "main.rs")
                && suite_dir(path).is_some_and(|dir| path.parent() == Some(dir.as_path()))
        })
        .map(|path| path.to_path_buf())
        .collect();
    let mut reached: BTreeSet<PathBuf> = queue.iter().cloned().collect();
    while let Some(file) = queue.pop() {
        let Some(text) = files.get(file.as_path()) else {
            continue;
        };
        for child in children(&file, text, &files) {
            if reached.insert(child.clone()) {
                queue.push(child);
            }
        }
    }
    files
        .iter()
        .filter(|(path, _)| path.extension().is_some_and(|ext| ext == "rs"))
        .filter(|(path, _)| suite_dir(path).is_some() && !reached.contains(**path))
        .filter(|(_, text)| {
            !text
                .lines()
                .take_while(|line| is_comment(line) || line.trim().is_empty())
                .any(|line| grants(line, RULE))
        })
        .map(|(path, _)| {
            let (module, declaration) = declarer(path, &files);
            Finding {
                path: path.to_path_buf(),
                line: None,
                rule: RULE,
                message: format!(
                    "no module declares it, so it is never compiled or run: add `{declaration}` \
                     to `{}` (or excuse it if it is brought in another way, such as `include!`)",
                    module.display()
                ),
            }
        })
        .collect()
}
