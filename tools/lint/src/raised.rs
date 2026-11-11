//! The files that may be longer than their kind's cap, by exact path.
//!
//! The size gate has no marker comment and no baseline: a file over its cap
//! is split. A raised cap is for the one shape a split cannot reach, a single
//! item that Rust cannot write across two files, and it is a **location**,
//! like [`crate::classify::is_skipped_dir`]'s rules, never a comment any file
//! could grow. Each entry is an exact path, a number and the reason, and the
//! number is still a cap: the file has to stay the one thing it is listed for.
//!
//! An entry is added only on the owner's decision (#1783 was the first), and
//! `tests/raised.rs` fails on one whose file has gone or whose reason is
//! missing.

use std::path::Path;

use crate::classify::Kind;

/// One file allowed past its kind's cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Raised {
    /// The file, relative to the repository root, with `/` separators.
    pub path: &'static str,
    /// Its cap, in lines of code.
    pub limit: usize,
    /// Why splitting it is not the answer.
    pub why: &'static str,
}

/// Every raised cap in the repository.
pub const RAISED: &[Raised] = &[Raised {
    path: "crates/zimmer/src/error/mod.rs",
    limit: 450,
    why: "`SynthError` is one flat public enum that callers and rusty match by \
          variant name, and an enum cannot be split across files; its words \
          already live in `error/wording/`, so this file holds the variants \
          and nothing else (#1783)",
}];

/// The cap `path` is held to, given the `kind` it was classified as: a raised
/// one when it is listed, its kind's otherwise.
pub fn limit_for(path: &Path, kind: Kind) -> usize {
    raised(path).map_or(kind.limit(), |entry| entry.limit)
}

/// The entry for `path`, if it has one.
pub fn raised(path: &Path) -> Option<&'static Raised> {
    RAISED.iter().find(|entry| Path::new(entry.path) == path)
}
