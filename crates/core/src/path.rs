//! Paths as they appear inside `project.json`.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A path stored in `project.json`: **relative to the project root**, written
/// with forward slashes on every platform.
///
/// This is the rule that lets a project survive `scp -r` between machines, so
/// it is not negotiable — but it is enforced by [`crate::Project::validate`], not by
/// this type's `Deserialize`. That is deliberate: a project with three bad
/// paths should report all three at once instead of aborting on the first.
/// Construction is therefore infallible; [`ProjectPath::check`] is the rule.
///
/// **Containment is the type's, not each caller's.** [`ProjectPath::resolve`]
/// applies [`ProjectPath::check`] itself, so a path that breaks the rules
/// never becomes a filesystem path, whatever wrote it (an unvalidated
/// document, a name built from an asset id) and whether or not its caller
/// checked it first.
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

    /// The path as stored: project-relative, forward slashes, whatever the
    /// host platform's own separator happens to be.
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

    /// Joins this path onto a project root to get a real filesystem path —
    /// or refuses, when it breaks the rules [`ProjectPath::check`] holds and
    /// could name something outside `project_root`.
    pub fn resolve(&self, project_root: &Path) -> Result<PathBuf, UnresolvablePath> {
        self.check().map_err(|problem| UnresolvablePath {
            path: self.clone(),
            problem,
        })?;
        Ok(self
            .0
            .split('/')
            .fold(project_root.to_path_buf(), |acc, s| acc.join(s)))
    }
}

/// A [`ProjectPath`] that [`ProjectPath::resolve`] refused: it names no file
/// inside the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvablePath {
    /// The path as written.
    pub path: ProjectPath,
    /// The rule it breaks.
    pub problem: PathProblem,
}

impl fmt::Display for UnresolvablePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` {}", self.path, self.problem)
    }
}

impl std::error::Error for UnresolvablePath {}

/// For a caller whose failures are the filesystem's: a path that names no file
/// in the project is, to it, input it cannot open.
impl From<UnresolvablePath> for std::io::Error {
    fn from(refused: UnresolvablePath) -> Self {
        Self::new(std::io::ErrorKind::InvalidInput, refused)
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
        // `pad` rather than `write_str`, so `{:<20}` in a table actually aligns.
        f.pad(&self.0)
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
