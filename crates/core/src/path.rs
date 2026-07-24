//! Paths as they appear inside `project.json`.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A path stored in `project.json`: **relative to the project root**, written
/// with forward slashes on every platform.
///
/// This is the rule that lets a project survive `scp -r` between machines, so
/// it is not negotiable — but it is enforced by [`crate::validate`], not by
/// this type's `Deserialize`. That is deliberate: a project with three bad
/// paths should report all three at once instead of aborting on the first.
/// Construction is therefore infallible; [`ProjectPath::check`] is the rule.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectPath(String);

/// Why a [`ProjectPath`] is not a legal project-relative path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathProblem {
    /// The path is empty.
    Empty,
    /// The path is absolute (`/media/clip.mp4`, `C:\clip.mp4`, `\\host\share`).
    Absolute,
    /// The path contains a backslash; `project.json` always uses `/`.
    Backslash,
    /// The path contains a `..` component and could escape the project root.
    ParentEscape,
}

impl ProjectPath {
    /// Wraps a string as a project path. Does not check the rules — see
    /// [`ProjectPath::check`].
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// `Ok(())` when this path obeys the project-relative rules.
    pub fn check(&self) -> Result<(), PathProblem> {
        if self.0.is_empty() {
            return Err(PathProblem::Empty);
        }
        if self.0.contains('\\') {
            return Err(PathProblem::Backslash);
        }
        if is_absolute(&self.0) {
            return Err(PathProblem::Absolute);
        }
        if self.0.split('/').any(|segment| segment == "..") {
            return Err(PathProblem::ParentEscape);
        }
        Ok(())
    }

    /// Joins this path onto a project root to get a real filesystem path.
    pub fn resolve(&self, project_root: &Path) -> PathBuf {
        self.0
            .split('/')
            .fold(project_root.to_path_buf(), |acc, s| acc.join(s))
    }
}

/// Absolute in the POSIX sense, the Windows drive sense, or the UNC sense.
/// All three are checked on every platform: a project written on Windows must
/// be rejected the same way when it is read on Linux.
fn is_absolute(path: &str) -> bool {
    if path.starts_with('/') || path.starts_with("\\\\") {
        return true;
    }
    let mut chars = path.chars();
    matches!((chars.next(), chars.next()), (Some(c), Some(':')) if c.is_ascii_alphabetic())
}

impl fmt::Display for ProjectPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for PathProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reason = match self {
            Self::Empty => "is empty",
            Self::Absolute => "is absolute; paths must be relative to the project root",
            Self::Backslash => "contains a backslash; use forward slashes",
            Self::ParentEscape => "contains a `..` component and escapes the project root",
        };
        f.write_str(reason)
    }
}
