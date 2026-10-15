//! The conventions a machine can hold, beside the size gate.
//!
//! CLAUDE.md: *a convention a machine can check is a gate, not a sentence*.
//! Each rule here is one written rule that nothing held until #1229:
//!
//! - [`prefix`] — group by subfolder, not filename prefix.
//! - [`markers`] — nothing in the codebase is temporary.
//! - [`skips`] — a test runs or fails, never skips; the database tests read
//!   their own variable, never the server's.
//! - [`crate_docs`] — each crate's module doc names its public modules.
//!
//! **Exceptions are inline, with a written reason** ([`allow`]): a comment
//! `lint: allow(<rule>) <why>` on the line a rule flags or in the comments
//! directly above it. Never a list in a config file nobody reads.

pub mod allow;
pub mod crate_docs;
pub mod markers;
pub mod prefix;
pub mod skips;

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::tree::code_files;

/// One place a rule says is off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The file, or for [`prefix`] the directory, relative to the scan root.
    pub path: PathBuf,
    /// The 1-based line, when the finding is about one.
    pub line: Option<usize>,
    /// The rule's name, which is also what an `allow` names.
    pub rule: &'static str,
    /// What is off and what to do about it.
    pub message: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(f, "{}:{line}: ", self.path.display())?,
            None => write!(f, "{}: ", self.path.display())?,
        }
        write!(f, "[{}] {}", self.rule, self.message)
    }
}

/// Every finding of every rule under `root`, ordered by path then line.
pub fn check_tree(root: &Path) -> io::Result<Vec<Finding>> {
    let files = code_files(root)?;
    let mut texts = Vec::with_capacity(files.len());
    for file in &files {
        let bytes = fs::read(root.join(&file.path))?;
        texts.push((file, String::from_utf8_lossy(&bytes).into_owned()));
    }
    let mut findings = prefix::check(texts.iter().map(|(file, text)| (&file.path, text.as_str())));
    for (file, text) in &texts {
        findings.extend(markers::check(&file.path, text));
        findings.extend(skips::check(&file.path, text));
        findings.extend(crate_docs::check(&file.path, text));
    }
    findings.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
    Ok(findings)
}
