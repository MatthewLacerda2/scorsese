//! The web app's server logs through `tracing`, never a print macro (#1520).
//!
//! A line from `eprintln!` has no level, no request and no job around it, so
//! nothing ties it to who asked or how long it took. Under `crates/server/src`
//! a print macro is flagged; the CLI and the stdio MCP server print to their
//! user on purpose and are not read.

use std::path::Path;

use super::Finding;
use super::allow::{allowed, is_comment};

/// The rule's name, as an `allow` names it.
pub const RULE: &str = "server-print";

/// The macros that write past the server's log.
const PRINTS: [&str; 5] = ["eprintln", "println", "eprint", "print", "dbg"];

/// `line`'s code: string literals blanked and a trailing `//` comment cut,
/// so `"http://x"` hides no call after it and a string holds none.
fn code(line: &str) -> String {
    let mut code = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut quoted = false;
    while let Some(c) = chars.next() {
        match (quoted, c) {
            (true, '\\') => {
                chars.next();
            }
            (true, '"') => {
                quoted = false;
                code.push('"');
            }
            (true, _) => {}
            (false, '"') => {
                quoted = true;
                code.push('"');
            }
            // A quote as a character, `'"'`, opens no string.
            (false, '\'') if chars.peek() == Some(&'"') => {
                chars.next();
                chars.next();
            }
            (false, '/') if chars.peek() == Some(&'/') => break,
            (false, _) => code.push(c),
        }
    }
    code
}

/// Whether `line` calls one of [`PRINTS`] as a name of its own.
fn prints(line: &str) -> bool {
    let code = code(line);
    PRINTS.iter().any(|name| {
        let call = format!("{name}!(");
        code.match_indices(&call).any(|(at, _)| {
            !code[..at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        })
    })
}

/// Every print macro in a server source file that no allow excuses.
pub fn check(path: &Path, text: &str) -> Vec<Finding> {
    if !path.starts_with("crates/server/src") || path.extension().is_none_or(|ext| ext != "rs") {
        return Vec::new();
    }
    let lines: Vec<&str> = text.lines().collect();
    lines
        .iter()
        .enumerate()
        .filter(|(index, line)| !is_comment(line) && prints(line) && !allowed(&lines, *index, RULE))
        .map(|(index, _)| Finding {
            path: path.to_path_buf(),
            line: Some(index + 1),
            rule: RULE,
            message: "the server logs through `tracing`: use `error!`, `warn!` or `info!`, \
                      or allow it inline with the reason it must print"
                .into(),
        })
        .collect()
}
