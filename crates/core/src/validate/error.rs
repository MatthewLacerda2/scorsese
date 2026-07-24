//! What validation can find, and how it reads.

use std::fmt;

use crate::asset::{AssetId, AssetKind};
use crate::keyframe::PropertyPath;
use crate::path::{PathProblem, ProjectPath};
use crate::timeline::{ClipId, TrackId, TrackKind};

/// One thing wrong with a project.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ValidationError {
    #[error("schema_version is {found}, but this build of scorsese reads {supported}")]
    UnsupportedSchemaVersion { found: u32, supported: u32 },

    #[error("asset id `{id}` is used more than once")]
    DuplicateAssetId { id: AssetId },

    #[error("asset `{asset}`: path `{path}` {problem}")]
    BadPath {
        asset: AssetId,
        path: ProjectPath,
        problem: PathProblem,
    },

    #[error("asset `{asset}` is a {kind:?} and needs a `path`")]
    MissingPath { asset: AssetId, kind: AssetKind },

    #[error("asset `{asset}` is a text asset and needs `text` content")]
    MissingText { asset: AssetId },

    #[error("asset `{asset}` is a {kind:?}, so `text` does not apply to it")]
    TextOnNonTextAsset { asset: AssetId, kind: AssetKind },

    #[error("asset `{asset}` is a {kind:?}, so `prompt` does not apply to it")]
    PromptOnPlainAsset { asset: AssetId, kind: AssetKind },

    #[error("asset `{asset}` is a {kind:?}, so `state` does not apply to it")]
    StateOnPlainAsset { asset: AssetId, kind: AssetKind },

    #[error("asset `{asset}` is a {kind:?} and needs a `prompt`")]
    MissingPrompt { asset: AssetId, kind: AssetKind },

    #[error("asset `{asset}` is a {kind:?} and needs a `state`")]
    MissingState { asset: AssetId, kind: AssetKind },

    #[error("asset `{asset}` is in state `generated` but has no `path` to the generated file")]
    GeneratedWithoutPath { asset: AssetId },

    #[error("asset `{asset}`: sha256 `{value}` is not 64 lowercase hex characters")]
    BadSha256 { asset: AssetId, value: String },

    #[error("track id `{id}` is used more than once")]
    DuplicateTrackId { id: TrackId },

    #[error("clip id `{id}` is used more than once")]
    DuplicateClipId { id: ClipId },

    #[error("clip `{clip}` references asset `{asset}`, which is not in the assets table")]
    DanglingAssetRef { clip: ClipId, asset: AssetId },

    #[error("clip `{clip}` has zero duration and would render nothing")]
    ZeroDuration { clip: ClipId },

    #[error("clips `{first}` and `{second}` overlap on track `{track}`")]
    OverlappingClips {
        track: TrackId,
        first: ClipId,
        second: ClipId,
    },

    #[error("clip `{clip}` puts a {asset_kind:?} asset on a {track_kind:?} track `{track}`")]
    TrackKindMismatch {
        track: TrackId,
        track_kind: TrackKind,
        clip: ClipId,
        asset_kind: AssetKind,
    },

    #[error("clip `{clip}`: property path `{property}` is malformed")]
    MalformedPropertyPath {
        clip: ClipId,
        property: PropertyPath,
    },

    #[error("clip `{clip}`: keyframes for `{property}` are not in ascending time order")]
    UnsortedKeyframes {
        clip: ClipId,
        property: PropertyPath,
    },

    #[error("clip `{clip}`: keyframe track for `{property}` is empty")]
    EmptyKeyframeTrack {
        clip: ClipId,
        property: PropertyPath,
    },

    #[error("clip `{clip}`: keyframe value for `{property}` is not a finite number")]
    BadKeyframeValue {
        clip: ClipId,
        property: PropertyPath,
    },
}

/// Everything wrong with a project, collected in one pass.
///
/// Validation reports all problems together rather than stopping at the
/// first: an agent fixing a project unattended should see the whole list, not
/// discover it one round-trip at a time.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationErrors(Vec<ValidationError>);

impl ValidationErrors {
    pub(crate) fn new(errors: Vec<ValidationError>) -> Self {
        Self(errors)
    }

    pub fn as_slice(&self) -> &[ValidationError] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn into_vec(self) -> Vec<ValidationError> {
        self.0
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let plural = if self.0.len() == 1 {
            "problem"
        } else {
            "problems"
        };
        write!(f, "{} {plural} in this project:", self.0.len())?;
        for error in &self.0 {
            write!(f, "\n  - {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

impl IntoIterator for ValidationErrors {
    type Item = ValidationError;
    type IntoIter = std::vec::IntoIter<ValidationError>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
