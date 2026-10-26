//! Enough of a `Cargo.toml` to know a package's name and what it depends on.
//!
//! The lint has no dependencies on purpose (its `Cargo.toml` says why), so
//! this reads TOML by hand, the way `tests/policy.rs` does: comments and blank
//! lines are dropped, a line opening with `[` is a header (a key never does,
//! and a header may hold an `=`: `[target.'cfg(unix = "x")'.dependencies]`)
//! that opens a table, and a value spread over
//! several lines (an inline table or array left open) is skipped until it
//! closes, so its inner keys are never read as dependencies. A dotted key
//! (`foo.workspace = true`) names the crate before its first dot, and the
//! lines of one key, dotted or under `[dependencies.foo]`, are one dependency.

/// One dependency a manifest declares, in any of its dependency tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    /// The crate depended on: the `package = "…"` it renames, or its key (the
    /// part before the first dot of a dotted one).
    pub name: String,
    /// The 1-based line declaring it.
    pub line: usize,
}

/// What a manifest says about its package and its dependencies.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Manifest {
    /// `[package] name`, when the manifest has a package.
    pub name: Option<String>,
    /// Every dependency of every table: normal, dev, build, and per target.
    pub dependencies: Vec<Dependency>,
}

/// Whether a table header (without its brackets) is a dependency table: the
/// header of the table itself, and the one dependency it names, if it is
/// `dependencies.foo`.
fn dependency_table(header: &str) -> Option<(&str, Option<&str>)> {
    const TABLES: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];
    for table in TABLES {
        if header == table || header.ends_with(&format!(".{table}")) {
            return Some((header, None));
        }
        if let Some((before, named)) = header.rsplit_once('.')
            && (before == table || before.ends_with(&format!(".{table}")))
        {
            return Some((before, Some(named)));
        }
    }
    None
}

/// The parts of a dotted key, each trimmed and unquoted: `foo.workspace` is
/// `["foo", "workspace"]`, and a quoted part keeps its dots (`"foo.bar"` and
/// `'foo.bar'` are one part).
fn key_parts(key: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut rest = key.trim();
    while !rest.is_empty() {
        let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'');
        let (part, after) = match quote.and_then(|q| rest[1..].split_once(q)) {
            Some((inside, after)) => (inside, after),
            None => rest
                .split_once('.')
                .map_or((rest, ""), |(part, _)| (part, &rest[part.len()..])),
        };
        parts.push(part.trim());
        let after = after.trim_start();
        rest = after.strip_prefix('.').unwrap_or(after).trim_start();
    }
    parts
}

/// The string a value opens with, in double or single quotes: `"1" # two` is
/// `1`, and a value that is not a string is `None`.
fn string_value(value: &str) -> Option<&str> {
    let value = value.trim_start();
    let quote = value.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    value[1..].split_once(quote).map(|(inside, _)| inside)
}

/// The string after `key =` in `value`, in double or single quotes.
fn quoted_after<'a>(value: &'a str, key: &str) -> Option<&'a str> {
    let at = value.find(key)?;
    string_value(value[at + key.len()..].trim_start().strip_prefix('=')?)
}

/// `line` without its comment: everything from a `#` outside of a string.
fn without_comment(line: &str) -> &str {
    let mut quote = None;
    for (at, c) in line.char_indices() {
        match (c, quote) {
            ('"' | '\'', None) => quote = Some(c),
            (c, Some(open)) if c == open => quote = None,
            ('#', None) => return line[..at].trim_end(),
            _ => {}
        }
    }
    line
}

/// How far `line` opens (positive) or closes (negative) brackets and braces,
/// outside of strings and comments.
fn depth_change(line: &str) -> i32 {
    let mut change = 0;
    let mut quote = None;
    for c in without_comment(line).chars() {
        match (c, quote) {
            ('"' | '\'', None) => quote = Some(c),
            (c, Some(open)) if c == open => quote = None,
            ('{' | '[', None) => change += 1,
            ('}' | ']', None) => change -= 1,
            _ => {}
        }
    }
    change
}

/// Records a dependency once per table and key, and says where it is: a later
/// line of the same dotted key (`foo.version`, then `foo.features`) adds
/// nothing but may rename it, and `renamed` is the `package = "…"` it gives.
fn record(
    manifest: &mut Manifest,
    keys: &mut Vec<(String, String)>,
    (table, key): (&str, &str),
    renamed: Option<&str>,
    line: usize,
) -> usize {
    let at = keys.iter().position(|(t, k)| t == table && k == key);
    let at = at.unwrap_or_else(|| {
        keys.push((table.to_string(), key.to_string()));
        manifest.dependencies.push(Dependency {
            name: key.to_string(),
            line,
        });
        keys.len() - 1
    });
    if let Some(renamed) = renamed {
        manifest.dependencies[at].name = renamed.to_string();
    }
    at
}

/// Reads a manifest's text.
pub fn parse(text: &str) -> Manifest {
    let mut manifest = Manifest::default();
    let mut keys = Vec::new();
    let mut table = String::new();
    // The dependency a `[dependencies.foo]` table names, while in it.
    let mut named = None;
    let mut open = 0;
    for (index, raw) in text.lines().enumerate() {
        let line = without_comment(raw.trim());
        if open > 0 {
            open += depth_change(line);
            continue;
        }
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            table = line
                .trim_matches(|c| c == '[' || c == ']')
                .trim()
                .to_string();
            named = match dependency_table(&table) {
                Some((deps, Some(key))) => {
                    let key = key_parts(key).first().copied().unwrap_or(key);
                    Some(record(
                        &mut manifest,
                        &mut keys,
                        (deps, key),
                        None,
                        index + 1,
                    ))
                }
                _ => None,
            };
            continue;
        }
        open = depth_change(line).max(0);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let parts = key_parts(key);
        let Some(&first) = parts.first() else {
            continue;
        };
        if table == "package" && parts == ["name"] {
            manifest.name = string_value(value).map(str::to_string);
            continue;
        }
        if let Some(at) = named {
            if let (["package"], Some(renamed)) = (&parts[..], string_value(value)) {
                manifest.dependencies[at].name = renamed.to_string();
            }
        } else if let Some((deps, None)) = dependency_table(&table) {
            let renamed = match parts.get(1) {
                Some(&"package") => string_value(value),
                Some(_) => None,
                None => quoted_after(value, "package"),
            };
            record(&mut manifest, &mut keys, (deps, first), renamed, index + 1);
        }
    }
    manifest
}
