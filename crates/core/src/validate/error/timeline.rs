//! What the track-and-clip checks can find: identity, references, time,
//! overlap, and keyframe shape.

use crate::asset::{AssetId, AssetKind};
use crate::keyframe::PropertyPath;
use crate::time::Frames;
use crate::timeline::{ClipId, TrackId, TrackKind};

/// One thing wrong with a track, a clip, or how a clip sits against the asset
/// it shows.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TimelineProblem {
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

    /// A clip showing source that is not there: `source_in` plus what the clip
    /// plays through reaches past the end of the media.
    ///
    /// The temporal sibling of [`TimelineProblem::CropOutsideSource`], and
    /// reported for the same reason — an edit that asks for footage nobody
    /// shot is an edit that went wrong, whichever axis it runs off. Reported
    /// only when the asset carries a **measured** length: a still, a title, a
    /// colour and a sketch have none to run out of, and a file nobody has
    /// probed has none *yet*. A ceiling invented from an absence would refuse
    /// honest edits on assets we simply have not looked at.
    #[error("clip `{clip}` reaches {reaches} into asset `{asset}`, which is {available} long")]
    ClipOutlastsSource {
        /// The clip asking for more than there is.
        clip: ClipId,
        /// The asset that runs out under it.
        asset: AssetId,
        /// The source frame the clip plays up to, `source_in` included.
        reaches: Frames,
        /// How much source there is, on the timeline's grid.
        available: Frames,
    },

    /// A `speed` that is not a rate anything could play at: zero, negative, or
    /// not a number.
    ///
    /// The float sibling of [`TimelineProblem::ZeroDuration`]. A frame count
    /// cannot be negative or fractional and so fails the parse, but every one of
    /// these fits in a float perfectly well — which is why the check is here,
    /// where it can name the clip.
    #[error("clip `{clip}` has a speed of {speed}, which is not a rate it can play at")]
    UnusableSpeed {
        /// The clip carrying it.
        clip: ClipId,
        /// The rate as written, so the message quotes the document rather than
        /// describing it.
        speed: f64,
    },

    /// A crop rectangle that runs off an edge of the source, or encloses none
    /// of it. Fractions are what make this checkable without knowing the
    /// source's pixel size — every edge lies in `0..=1`, and `x + width` and
    /// `y + height` cannot pass `1`.
    #[error("clip `{clip}` has a crop outside its source: {rectangle}")]
    CropOutsideSource {
        /// The clip carrying it.
        clip: ClipId,
        /// The rectangle as written, each edge named the way `project.json`
        /// spells it — so the message points at a field rather than at a
        /// description of one.
        rectangle: String,
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

    /// A keyframe track claiming a tool wrote it, and naming no tool.
    ///
    /// Worse than being unsigned: nothing can recognise it, so no tool will
    /// ever replace it and no reader can tell whose it is.
    #[error("clip `{clip}`: the `{property}` track has a blank `by`")]
    BlankKeyframeAuthor {
        /// The clip carrying it.
        clip: ClipId,
        /// Which track.
        property: PropertyPath,
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
