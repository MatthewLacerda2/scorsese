//! Walking a directory and measuring what [`classify`] says to measure.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::classify::{Kind, classify, is_skipped_dir};

/// One file over its limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// Relative to the scan root, so the message is the same everywhere.
    pub path: PathBuf,
    /// What the file actually measures.
    pub lines: usize,
    /// The cap it broke.
    pub kind: Kind,
}

impl fmt::Display for Violation {
    /// The whole point of the gate is this line, so it names the file, its
    /// length, and which of the two limits it broke. It says "lines of code"
    /// because the number will not match `wc -l`, and a reader comparing the
    /// two should be told why rather than left to discover it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} lines of code, over the {}-line limit for {} files (by {})",
            self.path.display(),
            self.lines,
            self.kind.limit(),
            self.kind.label(),
            self.lines - self.kind.limit(),
        )
    }
}

/// What one run of the gate found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// How many files were measured. A run that checked nothing is a broken
    /// run, not a passing one, so the count is reported rather than implied.
    pub checked: usize,
    /// Files over their limit, ordered by path.
    pub violations: Vec<Violation>,
}

impl Report {
    /// Whether the gate passes.
    pub fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Lines of code in a file's bytes: every line that is neither blank nor a
/// comment. A final unterminated line still counts, the same as any other.
///
/// The gate asks whether a file is doing too much, and what grows when it is
/// doing too much is its code. Documentation grows with how carefully each item
/// is explained — which this repo asks for on purpose, since `missing_docs` is
/// itself a merge gate and the house style is to say *why*. Counting both put
/// those two rules in opposition, and the prose lost: files were being split at
/// ~120 lines of code because their documentation had carried them to 300.
///
/// Splitting is a poor answer to prose volume in any case. It does not reduce
/// what has to be read, it distributes it and adds a module doc; it only pays
/// off when the seam is real, and a seam chosen by a line count often is not.
///
/// A comment is a line whose first non-whitespace is `//`, which covers `///`
/// and `//!` too. Block comments are not recognised: there are none in this
/// repo, and reading them would mean tracking state across lines for a case
/// that has never come up.
pub fn code_lines(bytes: &[u8]) -> usize {
    bytes.split(|&b| b == b'\n').filter(|l| is_code(l)).count()
}

/// Whether one line, without its terminator, counts toward the limit.
///
/// Trimming is what makes a `\r` from a Windows ending, and the indentation in
/// front of a doc comment, both invisible here.
fn is_code(line: &[u8]) -> bool {
    let line = line.trim_ascii();
    !line.is_empty() && !line.starts_with(b"//")
}

/// Measure every file under `root` the gate has an opinion about.
pub fn check_dir(root: &Path) -> io::Result<Report> {
    let mut report = Report::default();
    let mut dirs = vec![PathBuf::new()];

    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(root.join(&dir))? {
            let entry = entry?;
            let relative = dir.join(entry.file_name());
            // Symlinks are not followed: a link into a directory already being
            // walked would count its files twice, or loop.
            let file_type = entry.file_type()?;

            if file_type.is_dir() {
                let skip = entry.file_name().to_str().is_none_or(is_skipped_dir);
                if !skip {
                    dirs.push(relative);
                }
            } else if file_type.is_file()
                && let Some(kind) = classify(&relative)
            {
                report.checked += 1;
                let lines = code_lines(&fs::read(entry.path())?);
                if lines > kind.limit() {
                    report.violations.push(Violation {
                        path: relative,
                        lines,
                        kind,
                    });
                }
            }
        }
    }

    report.violations.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(report)
}
