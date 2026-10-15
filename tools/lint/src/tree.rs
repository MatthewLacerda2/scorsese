//! Walking the repository: every file the lint has an opinion about.
//!
//! One walk, shared by the size gate and the [`crate::rules`], so "which files
//! are looked at" has one answer. [`crate::classify::is_skipped_dir`] decides
//! what is never descended into; [`crate::classify`] which files are code.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::classify::{Kind, classify, is_skipped_dir};

/// One code file under the root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeFile {
    /// Relative to the scan root, so every message is the same everywhere.
    pub path: PathBuf,
    /// The cap it is held to, which is also whether it is a test.
    pub kind: Kind,
}

/// Every code file under `root`, ordered by path.
///
/// Symlinks are not followed: a link into a directory already being walked
/// would count its files twice, or loop.
pub fn code_files(root: &Path) -> io::Result<Vec<CodeFile>> {
    let mut found = Vec::new();
    let mut dirs = vec![PathBuf::new()];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(root.join(&dir))? {
            let entry = entry?;
            let relative = dir.join(entry.file_name());
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                if !is_skipped_dir(&relative) {
                    dirs.push(relative);
                }
            } else if file_type.is_file()
                && let Some(kind) = classify(&relative)
            {
                found.push(CodeFile {
                    path: relative,
                    kind,
                });
            }
        }
    }
    found.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(found)
}
