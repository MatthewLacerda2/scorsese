//! # scorsese-render — ffmpeg orchestration
//!
//! Responsibility: everything that touches ffmpeg/ffprobe — probing imported
//! media, decoding sources to raw frames over pipes, piping composited raw
//! frames into an ffmpeg encode process, and render settings (aspect,
//! resolution, fps, bitrate — user-chosen per render).
//!
//! Every ffmpeg invocation in the entire workspace goes through this crate's
//! command builder ([`tools::Tools`]). No ad-hoc `Command::new("ffmpeg")`
//! anywhere else, ever. In dev/CI ffmpeg is an external binary on PATH; in
//! shipped builds it is a bundled Tauri sidecar — this crate is the one place
//! that indirection lives.
//!
//! Boundary: no compositing logic (that is `scorsese-compositor`'s job — this
//! crate moves bytes, it never draws), no provider calls, no GUI. Depends on
//! `scorsese-core` and `scorsese-compositor`.

pub mod probe;
pub mod tools;

pub use probe::Ffprobe;
pub use tools::{Tools, ToolsError};
