//! Enough of a `Cargo.toml` to know a package's name and what it depends on.
//!
//! The lint has no dependencies on purpose (its `Cargo.toml` says why), so
//! this reads TOML by hand, the way `tests/policy.rs` does: comments and blank
//! lines are dropped, a `[header]` opens a table, and a value spread over
//! several lines (an inline table or array left open) is skipped until it
//! closes, so its inner keys are never read as dependencies.

/// One dependency a manifest declares, in any of its dependency tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    /// The crate depended on: the `package = "…"` it renames, or its key.
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

/// Whether a table header (without its brackets) is a dependency table, and
/// if it names one dependency (`dependencies.foo`), which.
fn dependency_table(header: &str) -> Option<Option<&str>> {
    const TABLES: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];
    for table in TABLES {
        if header == table || header.ends_with(&format!(".{table}")) {
            return Some(None);
        }
        if let Some((before, named)) = header.rsplit_once('.')
            && (before == table || before.ends_with(&format!(".{table}")))
        {
            return Some(Some(named));
        }
    }
    None
}

/// The text inside the first pair of double quotes after `key =` in `value`.
fn quoted_after<'a>(value: &'a str, key: &str) -> Option<&'a str> {
    let at = value.find(key)?;
    let rest = value[at + key.len()..].trim_start().strip_prefix('=')?;
    let rest = rest.trim_start().strip_prefix('"')?;
    rest.split_once('"').map(|(inside, _)| inside)
}

/// How far `line` opens (positive) or closes (negative) brackets and braces,
/// outside of double-quoted strings.
fn depth_change(line: &str) -> i32 {
    let mut quoted = false;
    let mut change = 0;
    for c in line.chars() {
        match c {
            '"' => quoted = !quoted,
            '{' | '[' if !quoted => change += 1,
            '}' | ']' if !quoted => change -= 1,
            '#' if !quoted => break,
            _ => {}
        }
    }
    change
}

/// Reads a manifest's text.
pub fn parse(text: &str) -> Manifest {
    let mut manifest = Manifest::default();
    let mut table = String::new();
    let mut open = 0;
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if open > 0 {
            open += depth_change(line);
            continue;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && !line.contains('=') {
            table = line
                .trim_matches(|c| c == '[' || c == ']')
                .trim()
                .to_string();
            if let Some(Some(named)) = dependency_table(&table) {
                manifest.dependencies.push(Dependency {
                    name: named.trim_matches('"').to_string(),
                    line: index + 1,
                });
            }
            continue;
        }
        open = depth_change(line).max(0);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().trim_matches('"');
        if table == "package" && key == "name" {
            manifest.name = quoted_after(line, "name").map(str::to_string);
        } else if dependency_table(&table) == Some(None) {
            let name = quoted_after(value, "package").unwrap_or(key);
            manifest.dependencies.push(Dependency {
                name: name.to_string(),
                line: index + 1,
            });
        }
    }
    manifest
}
