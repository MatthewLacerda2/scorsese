//! Every file ffmpeg reads is named through `scorsese_render::tools::input`
//! (#1635).
//!
//! A bare `-i <path>` lets ffmpeg read the name as a URL (`concat:a|b`) and
//! the file as a playlist naming others; the helper whitelists the file
//! protocol and the media formats. Under the `src/` of the crates that run
//! ffmpeg — render, providers, server — a `"-i"` string literal is flagged.
//! An input that is not a file (a lavfi graph, a pipe of raw frames this
//! program writes) is excused inline with its reason.

use std::path::Path;

use super::Finding;
use super::allow::{allowed, is_comment};

/// The rule's name, as an `allow` names it.
pub const RULE: &str = "ffmpeg-input";

/// The source folders read.
const READ: [&str; 3] = [
    "crates/render/src",
    "crates/providers/src",
    "crates/server/src",
];

/// The string literals on `line`, outside a trailing `//` comment.
fn literals(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut chars = line.chars().peekable();
    let mut current: Option<String> = None;
    while let Some(c) = chars.next() {
        match (&mut current, c) {
            (Some(_), '\\') => {
                chars.next();
            }
            (Some(text), '"') => {
                found.push(std::mem::take(text));
                current = None;
            }
            (Some(text), _) => text.push(c),
            (None, '"') => current = Some(String::new()),
            // A quote as a character, `'"'`, opens no string.
            (None, '\'') if chars.peek() == Some(&'"') => {
                chars.next();
                chars.next();
            }
            (None, '/') if chars.peek() == Some(&'/') => break,
            (None, _) => {}
        }
    }
    found
}

/// Every `"-i"` in a source file of a crate that runs ffmpeg that no allow
/// excuses.
pub fn check(path: &Path, text: &str) -> Vec<Finding> {
    if !READ.iter().any(|folder| path.starts_with(folder))
        || path.extension().is_none_or(|ext| ext != "rs")
    {
        return Vec::new();
    }
    let lines: Vec<&str> = text.lines().collect();
    lines
        .iter()
        .enumerate()
        .filter(|(index, line)| {
            !is_comment(line)
                && literals(line).iter().any(|literal| literal == "-i")
                && !allowed(&lines, *index, RULE)
        })
        .map(|(index, _)| Finding {
            path: path.to_path_buf(),
            line: Some(index + 1),
            rule: RULE,
            message: "name a file to ffmpeg with `scorsese_render::tools::input::media` (or \
                      `own` for a file this program wrote in a format it forces), never a bare \
                      `-i`; an input that is no file at all is allowed inline with what it is"
                .into(),
        })
        .collect()
}
