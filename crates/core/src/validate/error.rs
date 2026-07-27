//! What validation can find, and how it reads.

use std::fmt;

use crate::asset::{AssetId, AssetKind};
use crate::keyframe::PropertyPath;
use crate::path::{PathProblem, ProjectPath};
use crate::timeline::{ClipId, TrackId, TrackKind};

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

    /// Ids are how clips name assets, so a repeat makes every reference to it
    /// ambiguous.
    #[error("asset id `{id}` is used more than once")]
    DuplicateAssetId {
        /// The id claimed twice.
        id: AssetId,
    },

    /// A path that would not survive `scp -r` — absolute, backslashed, or
    /// climbing out of the project root.
    #[error("asset `{asset}`: path `{path}` {problem}")]
    BadPath {
        /// The asset carrying it.
        asset: AssetId,
        /// The path as written.
        path: ProjectPath,
        /// Which rule it breaks.
        problem: PathProblem,
    },

    /// Every kind but `text` is a file on disk, and there is no file without
    /// a path to it.
    #[error("asset `{asset}` is a {kind:?} and needs a `path`")]
    MissingPath {
        /// The asset with nothing to point at.
        asset: AssetId,
        /// The kind that requires a path.
        kind: AssetKind,
    },

    /// A text asset carries its content inline; without it there is nothing
    /// to draw.
    #[error("asset `{asset}` is a text asset and needs `text` content")]
    MissingText {
        /// The empty text asset.
        asset: AssetId,
    },

    /// Inline content on a file-backed asset — usually a `kind` that was
    /// meant to be `text`.
    #[error("asset `{asset}` is a {kind:?}, so `text` does not apply to it")]
    TextOnNonTextAsset {
        /// The asset with the stray field.
        asset: AssetId,
        /// The kind that has no use for it.
        kind: AssetKind,
    },

    /// Only the `generated_*` kinds are made from a prompt; a prompt anywhere
    /// else would never be acted on.
    #[error("asset `{asset}` is a {kind:?}, so `prompt` does not apply to it")]
    PromptOnPlainAsset {
        /// The asset with the stray field.
        asset: AssetId,
        /// The kind that has no use for it.
        kind: AssetKind,
    },

    /// The sketch lifecycle belongs to prompt-backed assets; an imported file
    /// is simply there.
    #[error("asset `{asset}` is a {kind:?}, so `state` does not apply to it")]
    StateOnPlainAsset {
        /// The asset with the stray field.
        asset: AssetId,
        /// The kind that has no lifecycle.
        kind: AssetKind,
    },

    /// A generated asset with no prompt is a bill nobody wrote the brief for.
    #[error("asset `{asset}` is a {kind:?} and needs a `prompt`")]
    MissingPrompt {
        /// The asset with nothing to generate from.
        asset: AssetId,
        /// The kind that requires a prompt.
        kind: AssetKind,
    },

    /// Without a state, GO cannot tell whether this has been paid for.
    #[error("asset `{asset}` is a {kind:?} and needs a `state`")]
    MissingState {
        /// The asset with no lifecycle position.
        asset: AssetId,
        /// The kind that requires a state.
        kind: AssetKind,
    },

    /// `generated` claims the media exists, so something has to say where.
    /// Usually a state edited by hand ahead of the generation.
    #[error("asset `{asset}` is in state `generated` but has no `path` to the generated file")]
    GeneratedWithoutPath {
        /// The asset claiming media it cannot produce.
        asset: AssetId,
    },

    /// Not the shape a SHA-256 comes in, so it can never match a real file —
    /// truncated, uppercase, or an algorithm that is not SHA-256.
    #[error("asset `{asset}`: sha256 `{value}` is not 64 lowercase hex characters")]
    BadSha256 {
        /// The asset carrying it.
        asset: AssetId,
        /// The string that is not a hash.
        value: String,
    },

    /// Two tracks answering to one name; reports about either become
    /// unreadable.
    #[error("track id `{id}` is used more than once")]
    DuplicateTrackId {
        /// The id claimed twice.
        id: TrackId,
    },

    /// Clip ids are unique across the whole project, not per track — they are
    /// what a render report names.
    #[error("clip id `{id}` is used more than once")]
    DuplicateClipId {
        /// The id claimed twice.
        id: ClipId,
    },

    /// The dangling reference: a clip with nothing to show. Usually an asset
    /// collected while a clip still pointed at it, or a mistyped id.
    #[error("clip `{clip}` references asset `{asset}`, which is not in the assets table")]
    DanglingAssetRef {
        /// The clip left pointing at nothing.
        clip: ClipId,
        /// The id that matched no asset.
        asset: AssetId,
    },

    /// A clip occupying no frames. Leaving a gap is how you say "nothing
    /// here"; a zero-length clip is an edit that went wrong.
    #[error("clip `{clip}` has zero duration and would render nothing")]
    ZeroDuration {
        /// The clip with no frames.
        clip: ClipId,
    },

    /// One track cannot show two things at once. Touching is fine — this is
    /// a shared frame, not a rounding tolerance.
    #[error("clips `{first}` and `{second}` overlap on track `{track}`")]
    OverlappingClips {
        /// The track they are both on.
        track: TrackId,
        /// The earlier of the two.
        first: ClipId,
        /// The one that starts before the first has ended.
        second: ClipId,
    },

    /// Picture on an audio track or sound on a video one. The track's kind
    /// decides what it can carry.
    #[error("clip `{clip}` puts a {asset_kind:?} asset on a {track_kind:?} track `{track}`")]
    TrackKindMismatch {
        /// The track the clip sits on.
        track: TrackId,
        /// What that track carries.
        track_kind: TrackKind,
        /// The misplaced clip.
        clip: ClipId,
        /// What its asset actually is.
        asset_kind: AssetKind,
    },

    /// An empty dotted segment (`a..b`, `.a`, `a.`) or an empty path. Not a
    /// claim that the property is unknown — core never checks that.
    #[error("clip `{clip}`: property path `{property}` is malformed")]
    MalformedPropertyPath {
        /// The clip holding the track.
        clip: ClipId,
        /// The path as written.
        property: PropertyPath,
    },

    /// Out of order, or two points at the same time. Left alone rather than
    /// sorted: only the author knows which of two values at one instant is
    /// the one meant.
    #[error("clip `{clip}`: keyframes for `{property}` are not in ascending time order")]
    UnsortedKeyframes {
        /// The clip holding the track.
        clip: ClipId,
        /// The property being animated.
        property: PropertyPath,
    },

    /// A track with no points animates nothing — the intent behind it was
    /// lost somewhere.
    #[error("clip `{clip}`: keyframe track for `{property}` is empty")]
    EmptyKeyframeTrack {
        /// The clip holding the track.
        clip: ClipId,
        /// The property with no points on it.
        property: PropertyPath,
    },

    /// A NaN or infinity. There is no value between it and its neighbour, so
    /// the ramp through it has no meaning.
    #[error("clip `{clip}`: keyframe value for `{property}` is not a finite number")]
    BadKeyframeValue {
        /// The clip holding the track.
        clip: ClipId,
        /// The property being animated.
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

    /// The problems, in the order validation found them: assets first, then
    /// tracks and clips.
    pub fn as_slice(&self) -> &[ValidationError] {
        &self.0
    }

    /// How many problems there are — what the `Display` line above the list
    /// counts.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// True when the project is valid. Never observed on a value that reached
    /// a caller: validation returns `Ok` rather than an empty error set.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Takes the problems out, for a caller that wants to sort or filter them
    /// rather than print them.
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
