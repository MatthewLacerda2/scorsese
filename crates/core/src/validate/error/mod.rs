//! What validation can find, and how it reads.
//!
//! One catalogue per thing being checked, matching the checks themselves: the
//! assets-table pass reports an [`AssetProblem`], the track-and-clip pass a
//! [`TimelineProblem`], and the version check sits on [`ValidationError`]
//! itself because it is about the document rather than about anything in it.
//! They collect into one list — a project's problems are reported together,
//! whatever part of the document they are about.
//!
//! Split because a catalogue grows with every check and the two halves grow
//! independently. Which half a new problem belongs in is decided the way the
//! checks are: by what has to be looked at to find it.

mod assets;
mod icon;
mod shape;
mod speech;
mod timeline;
mod video;

pub use assets::AssetProblem;
pub use icon::IconProblem;
pub use shape::ShapeProblem;
pub use speech::SpeechProblem;
pub use timeline::TimelineProblem;
pub use video::VideoProblem;

use crate::path::{PathProblem, ProjectPath};

/// One thing wrong with a project.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ValidationError {
    /// A document from another build of scorsese. Nothing else is worth
    /// reporting until this is settled.
    #[error("schema_version is {found}, but this build of scorsese reads {supported}")]
    UnsupportedSchemaVersion {
        /// The version the document declares.
        found: u32,
        /// The one version this build reads.
        supported: u32,
    },

    /// The document's `script` path, breaking the rules every other path
    /// obeys. It names no asset, so it is its own variant rather than a
    /// [`AssetProblem::BadPath`] with nothing to blame.
    ///
    /// The *shape* is all that is checked here. Whether the file is really
    /// there is `scorsese check`'s question and only ever a warning: a project
    /// that has lost its brief should still render.
    #[error("the project's script path `{path}` {problem}")]
    BadScriptPath {
        /// The path as written.
        path: ProjectPath,
        /// Which rule it breaks.
        problem: PathProblem,
    },

    /// Something wrong with a row of the assets table.
    #[error(transparent)]
    Asset(#[from] AssetProblem),

    /// Something wrong with a track, a clip, or how a clip sits against the
    /// asset it shows.
    #[error(transparent)]
    Timeline(#[from] TimelineProblem),
}

/// A video problem is an asset problem, so it reaches the collected list the
/// same way — spelled out because `From` does not chain, and a caller naming
/// the catalogue its subject lives in should not have to know how many layers
/// sit between that and the list.
impl From<VideoProblem> for ValidationError {
    fn from(problem: VideoProblem) -> Self {
        Self::Asset(problem.into())
    }
}

/// And a speech problem reaches it the same way, for the same reason.
impl From<SpeechProblem> for ValidationError {
    fn from(problem: SpeechProblem) -> Self {
        Self::Asset(problem.into())
    }
}

/// As does a shape problem.
impl From<ShapeProblem> for ValidationError {
    fn from(problem: ShapeProblem) -> Self {
        Self::Asset(problem.into())
    }
}

/// And an icon problem.
impl From<IconProblem> for ValidationError {
    fn from(problem: IconProblem) -> Self {
        Self::Asset(problem.into())
    }
}
