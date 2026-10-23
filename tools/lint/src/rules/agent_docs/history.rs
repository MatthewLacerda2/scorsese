//! An agent doc states the rule, never its history.
//!
//! A `CLAUDE.md` or a skill is loaded into every session, so a story, a date
//! or an issue number there is paid for by every reader and used by none: the
//! rule is what the agent acts on, and the issue and pull request that made it
//! are where its reasons are kept. Three shapes are found, in code spans too:
//!
//! - an issue or pull-request reference, `#123` or `owner/repo#123`;
//! - a GitHub link to one, `…/issues/123` or `…/pull/123`;
//! - a date, `2026-10-23`.
//!
//! A colour such as `#33ccff` is not a reference: its digits run into letters.
//! An HTML entity (`&#123;`) is not either.

use std::path::Path;

use super::super::Finding;
use super::super::allow::allowed;

/// The rule's name, as an `allow` names it.
pub const RULE: &str = "history";

/// What, if anything, on `line` cites history.
pub fn cites(line: &str) -> Option<&'static str> {
    if reference(line) {
        Some("an issue or pull-request number: state the rule; its issue keeps why")
    } else if link(line) {
        Some("a link to an issue or pull request: state the rule; its issue keeps why")
    } else if date(line) {
        Some("a date: state the rule as it stands now")
    } else {
        None
    }
}

/// `#` followed by digits that end the word, not after `&`.
fn reference(line: &str) -> bool {
    let bytes = line.as_bytes();
    bytes.iter().enumerate().any(|(i, &b)| {
        if b != b'#' || (i > 0 && bytes[i - 1] == b'&') {
            return false;
        }
        let digits = run(&bytes[i + 1..], u8::is_ascii_digit);
        digits > 0
            && !bytes
                .get(i + 1 + digits)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
    })
}

/// `/issues/` or `/pull/` followed by digits.
fn link(line: &str) -> bool {
    ["/issues/", "/pull/"].iter().any(|segment| {
        line.match_indices(segment)
            .any(|(at, _)| run(&line.as_bytes()[at + segment.len()..], u8::is_ascii_digit) > 0)
    })
}

/// `dddd-dd-dd`, not inside a longer run of digits.
fn date(line: &str) -> bool {
    let bytes = line.as_bytes();
    (0..bytes.len()).any(|i| {
        let shape = [4, 2, 2];
        let mut at = i;
        for (n, &width) in shape.iter().enumerate() {
            if run(&bytes[at..], u8::is_ascii_digit) != width {
                return false;
            }
            at += width;
            if n < 2 {
                if bytes.get(at) != Some(&b'-') {
                    return false;
                }
                at += 1;
            }
        }
        i == 0 || !bytes[i - 1].is_ascii_digit()
    })
}

/// How many leading bytes of `bytes` satisfy `test`.
fn run(bytes: &[u8], test: fn(&u8) -> bool) -> usize {
    bytes.iter().take_while(|b| test(b)).count()
}

/// A finding for every line of `text` that cites history and is not excused.
pub fn check(path: &Path, text: &str) -> Vec<Finding> {
    let lines: Vec<&str> = text.lines().collect();
    lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            let message = cites(line)?;
            (!allowed(&lines, index, RULE)).then(|| Finding {
                path: path.to_path_buf(),
                line: Some(index + 1),
                rule: RULE,
                message: message.into(),
            })
        })
        .collect()
}
