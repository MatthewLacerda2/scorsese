//! An asset's id, and what it may hold.
//!
//! An id is what a clip names an asset by, and also a file name: a paid
//! output lands at `generated/<id>-<brief hash>`, a partial bake at
//! `cache/synth/<id>.wav`, a page at `pages/<id>.html`. So an id holds only
//! what a file name keeps on every system and in every folder: ASCII letters,
//! digits, `-`, `_` and `.`, never a leading `.`, and at most
//! [`MAX_ASSET_ID_LENGTH`] of them.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The longest an asset id may be. A paid line's word timings are named
/// `<id>-<64 hex digits>.mp3.words.json`, which this keeps well inside the 255
/// bytes a file name may hold.
pub const MAX_ASSET_ID_LENGTH: usize = 128;

/// Identifies an asset within one project. Unique across the assets table.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetId(String);

impl AssetId {
    /// Wraps a string as an id. Neither the rule on its characters nor
    /// uniqueness is checked here: both are [`crate::Project::validate`]'s.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The id as written in `project.json`.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// `Ok(())` when the id keeps the rule an id keeps.
    pub fn check(&self) -> Result<(), AssetIdProblem> {
        let id = self.0.as_str();
        if id.is_empty() {
            return Err(AssetIdProblem::Empty);
        }
        if id.starts_with('.') {
            return Err(AssetIdProblem::LeadingDot);
        }
        if let Some(refused) = id.chars().find(|c| !allowed(*c)) {
            return Err(AssetIdProblem::Character(refused));
        }
        if id.len() > MAX_ASSET_ID_LENGTH {
            return Err(AssetIdProblem::TooLong);
        }
        Ok(())
    }
}

impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `pad` rather than `write_str`, so `{:<20}` in a table actually aligns.
        f.pad(&self.0)
    }
}

/// Whether `name` holds only an id's characters and does not start with a
/// dot — the rule without the length cap, for a name an id is the start of
/// (a batch request's key is `<id>-<brief hash>`).
pub fn is_plain_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && name.chars().all(allowed)
}

/// Why an [`AssetId`] breaks the rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetIdProblem {
    /// The id is empty.
    Empty,
    /// The id starts with `.`, as a hidden file and `..` do.
    LeadingDot,
    /// The id holds a character outside the set: the first such.
    Character(char),
    /// The id is longer than [`MAX_ASSET_ID_LENGTH`].
    TooLong,
}

impl fmt::Display for AssetIdProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("is empty"),
            Self::LeadingDot => f.write_str(
                "starts with `.`: an asset id is a file name, and one starting with a dot is \
                 hidden or leaves its folder",
            ),
            Self::Character(refused) => write!(
                f,
                "contains `{}`: an asset id holds only ASCII letters, digits, `-`, `_` and `.`",
                refused.escape_debug()
            ),
            Self::TooLong => write!(f, "is longer than {MAX_ASSET_ID_LENGTH} characters"),
        }
    }
}

fn allowed(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')
}

/// `raw` made into an id that keeps the rule: each run of characters it may
/// not hold becomes one `-` (none right after a `-`), leading dots and
/// hyphens and trailing hyphens go, nothing left is `asset`, and a name longer
/// than [`MAX_ASSET_ID_LENGTH`] is cut in its middle. Case is kept, and so is
/// every character the rule allows: an id that keeps the rule comes back as
/// it was.
pub(crate) fn tidy(raw: &str) -> String {
    let mut tidied = String::with_capacity(raw.len());
    for c in raw.chars() {
        if allowed(c) {
            tidied.push(c);
        } else if !tidied.ends_with('-') {
            tidied.push('-');
        }
    }
    let trimmed = tidied.trim_start_matches(['.', '-']).trim_end_matches('-');
    if trimmed.is_empty() {
        String::from("asset")
    } else {
        cut(trimmed)
    }
}

/// `name` at most [`MAX_ASSET_ID_LENGTH`] long, cut in its middle: its first
/// half and its last, joined by `-`. The end of a name is what tells `take-2`
/// from `take-3`, so a caller that suffixes a name itself still gets distinct
/// ids.
fn cut(name: &str) -> String {
    if name.len() <= MAX_ASSET_ID_LENGTH {
        return name.to_owned();
    }
    // Every character is ASCII by now, so any byte is a character boundary.
    let head = &name[..MAX_ASSET_ID_LENGTH / 2];
    let tail = &name[name.len() - (MAX_ASSET_ID_LENGTH / 2 - 1)..];
    format!("{head}-{tail}")
}

/// `wanted` if `taken` refuses it not, or the first of `wanted-2`,
/// `wanted-3`… that is free — `wanted` cut short where the suffix would run
/// past [`MAX_ASSET_ID_LENGTH`]. `wanted` keeps the rule (it is [`tidy`]'s),
/// and so does what comes back.
///
/// Bounded as [`crate::ids::numbered`] is: `taken` holds at most `limit` ids,
/// and the `limit + 1` candidates differ in the number after their last `-`.
pub(crate) fn unique(wanted: &str, limit: usize, taken: impl Fn(&str) -> bool) -> String {
    if !taken(wanted) {
        return wanted.to_owned();
    }
    (2..=limit + 2)
        .map(|number| {
            let suffix = format!("-{number}");
            let room = MAX_ASSET_ID_LENGTH - suffix.len();
            let base = wanted[..wanted.len().min(room)].trim_end_matches('-');
            format!("{base}{suffix}")
        })
        .find(|candidate| !taken(candidate))
        .expect("`limit` ids cannot take every one of `limit + 1` numbers")
}

#[cfg(test)]
mod tests {
    use super::{tidy, unique};

    #[test]
    fn tidying_replaces_each_run_trims_and_falls_back_to_asset() {
        assert_eq!(tidy("take: 2"), "take-2");
        assert_eq!(tidy("../../x"), "x");
        assert_eq!(tidy("%%%"), "asset");
        assert_eq!(tidy("Shot_1.v2"), "Shot_1.v2");
        assert_eq!(tidy("end -"), "end");
        assert_eq!(tidy("vo--1"), "vo--1", "hyphens already written stay");
    }

    /// A caller that suffixes a long name itself (`<stem>-2`, `<stem>-3`…)
    /// still gets a different id for each: the cut keeps the name's end.
    #[test]
    fn a_long_name_is_cut_in_the_middle_so_its_end_survives() {
        let stem = "x".repeat(200);
        let second = tidy(&format!("{stem}-2"));
        let third = tidy(&format!("{stem}-3"));
        assert_eq!(second.len(), super::MAX_ASSET_ID_LENGTH);
        assert!(second.ends_with("x-2"), "{second}");
        assert_ne!(second, third);
    }

    #[test]
    fn a_suffix_never_runs_past_the_cap() {
        let full = "a".repeat(super::MAX_ASSET_ID_LENGTH);
        let taken = [full.clone()];
        let found = unique(&full, 1, |id| taken.iter().any(|t| t == id));
        assert_eq!(found.len(), super::MAX_ASSET_ID_LENGTH);
        assert!(found.ends_with("a-2"), "{found}");
        assert_eq!(unique("free", 0, |_| false), "free");
    }
}
