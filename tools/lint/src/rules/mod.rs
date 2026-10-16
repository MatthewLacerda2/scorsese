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
//! - [`ignored`] — what you push is what CI builds: no code file `.gitignore`
//!   would leave out of a commit.
//! - [`schema`] — a Rust test document at this build's version interpolates
//!   `SCHEMA_VERSION` rather than writing the number.
//! - [`shebang`] — a tracked script with a shebang is executable, and an
//!   executable text file has a shebang (#1347).
//! - [`boundary`] — the decided architecture's boundaries: which crate may
//!   depend on which, who holds the HTTP and Postgres clients, a headless
//!   root workspace, and the words a crate's source may not write (#1224).
//!
//! **Exceptions are inline, with a written reason** ([`allow`]): a comment
//! `lint: allow(<rule>) <why>` on the line a rule flags or in the comments
//! directly above it. Never a list in a config file nobody reads.

pub mod allow;
pub mod boundary;
pub mod crate_docs;
pub mod ignored;
pub mod markers;
pub mod prefix;
pub mod schema;
pub mod shebang;
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
    let paths: Vec<&Path> = files.iter().map(|file| file.path.as_path()).collect();
    findings.extend(ignored::check(root, &paths));
    findings.extend(shebang::check(root));
    findings.extend(boundary::check_root(root)?);
    let current = texts
        .iter()
        .find(|(file, _)| file.path == Path::new(schema::DEFINED_IN))
        .and_then(|(_, text)| schema::defined_version(text));
    for (file, text) in &texts {
        if let Some(current) = current {
            findings.extend(schema::check(&file.path, text, current));
        }
        findings.extend(markers::check(&file.path, text));
        findings.extend(skips::check(&file.path, text));
        findings.extend(crate_docs::check(&file.path, text));
        findings.extend(boundary::fences::check(&file.path, text));
    }
    findings.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
    Ok(findings)
}
