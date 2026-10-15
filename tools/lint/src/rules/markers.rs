//! Nothing in the codebase is temporary (CLAUDE.md).
//!
//! A marker promising to come back later is the temporary thing in its
//! plainest form: the work it names is either done now or filed as an issue,
//! where it is planned and seen, never left in a comment nobody is assigned to.
//! The markers are the four in [`MARKERS`], as whole upper-case words.

use std::path::Path;

use super::Finding;
use super::allow::allowed;

/// The rule's name, as an `allow` names it.
pub const RULE: &str = "marker";

// lint: allow(marker) these are the words the rule looks for.
/// The words a line may not carry.
pub const MARKERS: [&str; 4] = ["TODO", "FIXME", "XXX", "HACK"];

/// The first marker on `line` standing as a word of its own, if any.
pub fn marker_in(line: &str) -> Option<&'static str> {
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    MARKERS.into_iter().find(|marker| {
        line.match_indices(marker).any(|(at, _)| {
            let before = line[..at].chars().next_back();
            let after = line[at + marker.len()..].chars().next();
            !before.is_some_and(is_word) && !after.is_some_and(is_word)
        })
    })
}

/// Every line of `text` carrying a marker that no allow excuses.
pub fn check(path: &Path, text: &str) -> Vec<Finding> {
    let lines: Vec<&str> = text.lines().collect();
    lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| Some((index, marker_in(line)?)))
        .filter(|(index, _)| !allowed(&lines, *index, RULE))
        .map(|(index, marker)| Finding {
            path: path.to_path_buf(),
            line: Some(index + 1),
            rule: RULE,
            message: format!("`{marker}`: do it now, or file an issue and drop the marker"),
        })
        .collect()
}
