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
//! Boundary: this crate must never touch a display, a GPU, a network, or
//! spawn a process. No GUI, no ffmpeg, no provider calls — it reads and
//! writes the project document and reasons about it. Everything else depends
//! on it.
//!
//! The format itself is documented in `docs/project-format.md`.

pub mod asset;
pub mod color;
pub mod dip;
pub mod keyframe;
pub mod path;
pub mod pool;
pub mod probe;
pub mod project;
pub mod text;
pub mod time;
pub mod timeline;
pub mod validate;
pub mod write;

pub use asset::{Asset, AssetId, AssetKind, GenerationState, MediaMetadata};
pub use color::{ColorError, Rgba};
pub use dip::{Dip, Ducked, Span, Under, duck_track};
pub use keyframe::{Easing, Keyframe, KeyframeTrack, PropertyPath};
pub use path::{PathProblem, ProjectPath};
pub use pool::{
    AssetHealth, AssetStatus, HashCheck, ImportError, asset_id_for, asset_status, hash_bytes,
    import_asset,
};
pub use probe::{ProbeError, ProbeMedia};
pub use project::{
    ASSETS_DIR, CACHE_DIR, GENERATED_DIR, LoadError, PROJECT_FILE_NAME, Project, RECIPES_DIR,
    SCHEMA_VERSION, SaveError,
};
pub use text::{FontChoice, MAX_WEIGHT, MIN_WEIGHT, TextAlign, TextStyle};
pub use time::{Fps, FpsError, FpsParseError, Frames};
pub use timeline::{Anchor, AnchorX, AnchorY, Clip, ClipId, Crop, Fit, Track, TrackId, TrackKind};
pub use validate::{AssetField, ValidationError, ValidationErrors};
