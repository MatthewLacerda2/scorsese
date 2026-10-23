//! Group by subfolder, not filename prefix.
//!
//! Three or more sibling code files whose names start with the same word —
//! `mutants-merge.py`, `mutants-scope.py`, `mutants-summary.py` — are a
//! subfolder waiting to happen: `mutants/merge.py` and its siblings say the
//! same thing once, in the path, where a reader browsing the tree sees it.
//!
//! The word is the name up to its first `-` or `_`, after any `test_` (which
//! `unittest` and pytest require, and so says nothing about grouping). Names
//! are counted once per stem — the part before the first `.` — so `card.tsx`
//! and `card.test.tsx` are one name, not two. A bare `word` counts with the
//! `word_*` beside it, since `mergeable.py`, `mergeable_picking.py` and
//! `mergeable_skipped.py` are the same smell; three bare stems sharing no
//! separator cannot happen, as stems are counted once.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::Finding;
use super::allow::grants;

/// The rule's name, as an `allow` names it.
pub const RULE: &str = "prefix-group";

/// How many names sharing a word make a group.
pub const GROUP: usize = 3;

/// A name's stem and its first word, or `None` when it has no word to share.
fn word_of(path: &Path) -> Option<(String, String)> {
    let name = path.file_name()?.to_str()?;
    let stem = name.split('.').next()?;
    let stem = stem.strip_prefix("test_").unwrap_or(stem);
    let word: String = stem
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    (!word.is_empty()).then(|| (stem.to_string(), word))
}

/// Every group of [`GROUP`] or more names sharing a first word in one
/// directory, unless one of its files allows it.
pub fn check<'a>(files: impl Iterator<Item = (&'a PathBuf, &'a str)>) -> Vec<Finding> {
    let mut groups: BTreeMap<(PathBuf, String), (BTreeSet<String>, bool)> = BTreeMap::new();
    for (path, text) in files {
        let Some((stem, word)) = word_of(path) else {
            continue;
        };
        let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
        let entry = groups.entry((dir, word)).or_default();
        entry.0.insert(stem);
        entry.1 |= text.lines().any(|line| grants(line, RULE));
    }
    groups
        .into_iter()
        .filter(|((_, word), (stems, allowed))| {
            !allowed && stems.len() >= GROUP && stems.iter().any(|stem| stem != word)
        })
        .map(|((dir, word), (stems, _))| Finding {
            message: format!(
                "{} names share the prefix `{word}` ({}): group them in a `{word}/` subfolder",
                stems.len(),
                stems.into_iter().collect::<Vec<_>>().join(", "),
            ),
            path: dir,
            line: None,
            rule: RULE,
        })
        .collect()
}
