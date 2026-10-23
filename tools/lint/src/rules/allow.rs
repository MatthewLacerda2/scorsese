//! The one way a rule is excused: inline, with a reason.
//!
//! `lint: allow(<rule>) <why>` in a comment, on the flagged line or in the run
//! of comment and attribute lines directly above it (so an allow can sit
//! above a `#[test]` the flagged attribute follows). The reason is required — an allow with
//! nothing after it excuses nothing — because the point of an inline exception
//! is that the next reader sees *why* beside *what*.

/// The text an allow starts with, before the rule's name.
pub const PREFIX: &str = "lint: allow(";

/// Whether `line` carries an allow for `rule` with a reason after it.
pub fn grants(line: &str, rule: &str) -> bool {
    let Some(start) = line.find(PREFIX) else {
        return false;
    };
    let rest = &line[start + PREFIX.len()..];
    let Some(after) = rest.strip_prefix(rule).and_then(|r| r.strip_prefix(')')) else {
        return false;
    };
    after
        .trim_start_matches(|c: char| c.is_whitespace() || matches!(c, '-' | ':' | '—'))
        .chars()
        .any(char::is_alphanumeric)
}

/// Whether a trimmed line is a comment in any language the lint reads: `//`
/// and `/* */` for Rust and TypeScript, `#` for Python (but not a Rust `#[`
/// or `#!` attribute), `<!--` for Markdown.
pub fn is_comment(line: &str) -> bool {
    let line = line.trim_start();
    line.starts_with("//")
        || line.starts_with("/*")
        || line.starts_with('*')
        || line.starts_with("<!--")
        || (line.starts_with('#') && !line.starts_with("#[") && !line.starts_with("#!"))
}

/// Whether line `index` (0-based) of `lines` is excused from `rule`: it, or a
/// comment line in the unbroken run of comments and Rust attributes directly
/// above it, grants an allow.
pub fn allowed(lines: &[&str], index: usize, rule: &str) -> bool {
    if lines.get(index).is_some_and(|line| grants(line, rule)) {
        return true;
    }
    lines[..index.min(lines.len())]
        .iter()
        .rev()
        .take_while(|line| is_comment(line) || line.trim_start().starts_with("#["))
        .any(|line| grants(line, rule))
}
