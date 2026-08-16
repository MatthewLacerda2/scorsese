//! # scorsese-core — the project model
//!
//! Responsibility: the timeline model, the assets table, clips, keyframe
//! tracks, the serde `project.json` format, and validation (dangling asset
//! references, overlap constraints, path hygiene).
//!
//! A project is a directory (`*.scor/`): `project.json` plus `assets/`,
//! `generated/`, `recipes/`, and `cache/`. Every path stored in
//! `project.json` is
//! relative to the project root — never absolute — so a project survives
//! `scp -r` between machines. Assets are entities (id, kind, path, sha256,
//! probed metadata); clips reference assets by id, never by path.
//!
//! The timeline is measured in **frames on an exact rational grid**: a
//! project carries a `timeline_fps`, and every clip and keyframe time is a
//! whole frame count on it. Seconds appear only at the edges — showing a time
//! to a human, handing one to ffmpeg. Output framerate stays a render
//! setting; a render at another rate conforms from this grid.
//!
//! Keyframe tracks are generic: `(property_path, [(t, value, easing)])` over
//! any numeric property. This crate defines property *types*, never property
//! *values*.
//!
//! A project also carries **why** it is the way it is: a `script` at the
//! document's root, and a `note` on anything with an `id`. Neither ever
//! reaches the screen — see [`note`] for what they are for and why they are
//! two mechanisms rather than one.
//!
//! Boundary: this crate must never touch a display, a GPU, a network, or
//! spawn a process. No GUI, no ffmpeg, no provider calls — it reads and
//! writes the project document and reasons about it. Everything else depends
//! on it.
//!
//! The format itself is documented in `docs/project-format.md`.
//!
//! ## What this publishes
//!
//! **The document, at the crate root.** Every type a `project.json` is made of
//! — [`Project`], [`Asset`] and [`AssetKind`], [`Track`], [`Clip`] and their
//! ids, [`KeyframeTrack`] and [`Easing`], [`Fps`], [`Frames`] and [`Speed`],
//! [`TextStyle`], [`Rgba`], [`ProjectPath`] — is `scorsese_core::Thing` and
//! nothing longer. Six crates and the desktop app all read and write the same
//! document, and a model type with two import paths is a model type that gets
//! spelled two ways. [`Baseline`] and [`fingerprint_of`] sit there too, one
//! step out from the rest: not a part of a `project.json`, but what a
//! [`Project`] remembers about the file it was read from, so that saving it
//! cannot quietly land on somebody else's edit.
//!
//! **Seven modules keep their path**, because what they publish is an
//! *operation* on a project rather than a part of one, and the verb needs the
//! noun in front of it: [`mod@pool`] brings media in, hashes it, probes it and
//! collects what nothing references; [`mod@placing`] puts a clip on a track and
//! moves or trims one already there; [`mod@pacing`] retimes a cut;
//! [`mod@dip`] is auto-ducking; [`mod@level`] holds one clip's property at a
//! value; [`mod@probe`] is the seam an ffprobe lives behind, so this crate can
//! reason about media without spawning anything; and [`mod@write`] is the one
//! way a file leaves here. [`note`] keeps its own as well — the paragraph
//! above sends the reader to it.
//!
//! **Everything else is `pub(crate)`.** How the document is parsed and saved,
//! how the frame grid does its arithmetic, how a font choice and a colour are
//! read, and the whole of validation's machinery behind [`Project::validate`]
//! — those are the steps this crate is made of, and nothing above it composes
//! them differently.

pub(crate) mod asset;
pub(crate) mod baseline;
pub(crate) mod color;
pub mod dip;
pub(crate) mod grade;
pub(crate) mod keyframe;
pub mod level;
pub mod note;
pub mod pacing;
pub(crate) mod path;
pub mod placing;
pub mod pool;
pub mod probe;
pub(crate) mod project;
pub(crate) mod shape;
pub(crate) mod stamp;
pub(crate) mod text;
pub(crate) mod time;
pub(crate) mod timeline;
pub(crate) mod validate;
pub mod write;

pub use asset::{
    Aspect, Asset, AssetId, AssetKind, ClipSeconds, GenerationState, LanguageIgnored, LengthLock,
    MAX_CHARACTERS, MAX_REFERENCE_IMAGES, MediaMetadata, SpeechModel, SpeechRequest, VideoModel,
    VideoRequest, VideoResolution,
};
pub use baseline::{Baseline, fingerprint_of};
pub use color::{ColorError, Rgba};
pub use dip::{Dip, Ducked, Under, duck_track};
pub use grade::Grade;
pub use keyframe::{Easing, Keyframe, KeyframeTrack, PropertyPath};
// As with `pacing::scale`, the function stays behind its module: a bare
// `scorsese_core::set` names nothing at all.
pub use level::{Level, LevelError, Levelled};
pub use note::{Annotated, Noted};
// The function itself is deliberately left behind the module — see
// [`pacing`] for why `scorsese_core::scale` would be the wrong name.
pub use pacing::{PaceError, Paced};
pub use path::{PathProblem, ProjectPath};
// The two functions stay behind the module, for [`pacing`]'s reason: a bare
// `scorsese_core::trim` says nothing about what it trims, and `place` reads as
// spatial.
pub use placing::{PlaceError, Placement, Trim, TrimError};
pub use pool::{
    AssetHealth, AssetStatus, HashCheck, Import, ImportError, Imported, ProbeOutcome, Probed,
    Reprobe, SkipReason, Skipped, asset_id_for, asset_status, hash_bytes, import_asset,
    import_path, probe_assets, unprobed_assets,
};
pub use probe::{ProbeError, ProbeMedia};
pub use project::{
    ASSETS_DIR, CACHE_DIR, GENERATED_DIR, LoadError, PROJECT_FILE_NAME, Project, RECIPES_DIR,
    SCHEMA_VERSION, SaveError,
};
pub use shape::{
    Attach, Curve, DEFAULT_STROKE_WIDTH, Endpoint, Geometry, Heads, MAX_RADIUS, Point, Shape, Side,
};
pub use stamp::{Timestamp, TimestampError};
pub use text::{DEFAULT_FONT, FontChoice, MAX_WEIGHT, MIN_WEIGHT, TextAlign, TextStyle};
pub use time::{Fps, FpsError, FpsParseError, Frames, Speed};
pub use timeline::{Anchor, AnchorX, AnchorY, Clip, ClipId, Crop, Fit, Track, TrackId, TrackKind};
pub use validate::{
    AssetField, AssetProblem, ShapeProblem, SpeechProblem, TimelineProblem, ValidationError,
    ValidationErrors, VideoProblem,
};
