//! The media pool: how media gets into a project, what state it is in, and
//! what can be thrown away.
//!
//! Everything here works on the project *directory* — copying files into
//! `assets/`, hashing them, deleting them — but never spawns a process. The
//! one thing that needs an external tool, probing, is injected as a
//! [`crate::probe::ProbeMedia`] so this crate keeps its boundary and so
//! importing stays testable without ffmpeg.

mod gc;
mod hash;
mod import;
mod naming;
mod status;

pub use gc::{GcError, GcReport, remove_assets, unused_assets};
pub use hash::{hash_bytes, hash_file};
pub use import::{ImportError, import_asset};
pub use naming::{asset_id_for, infer_kind};
pub use status::{AssetHealth, AssetStatus, HashCheck, asset_status};
