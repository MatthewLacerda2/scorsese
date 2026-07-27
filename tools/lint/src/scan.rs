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
    /// length, and which of the two limits it broke.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} lines, over the {}-line limit for {} files (by {})",
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

/// Lines in a file's bytes: newline-separated, with a final unterminated line
/// still counting as one. Blank and comment lines count like any other — what
/// is being capped is how much file there is to read.
pub fn line_count(bytes: &[u8]) -> usize {
    let newlines = bytes.iter().filter(|&&b| b == b'\n').count();
    if bytes.last().is_some_and(|&b| b != b'\n') {
        newlines + 1
    } else {
        newlines
    }
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
                let lines = line_count(&fs::read(entry.path())?);
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
