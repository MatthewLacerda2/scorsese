//! # scorsese-render — ffmpeg orchestration
//!
//! Responsibility: everything that touches ffmpeg/ffprobe — probing imported
//! media, decoding sources to raw frames over pipes, piping composited raw
//! frames into an ffmpeg encode process, and render settings (resolution, fps,
//! bitrate — user-chosen per render).
//!
//! Every ffmpeg invocation in the entire workspace goes through this crate's
//! command builder ([`tools::Tools`]). No ad-hoc `Command::new("ffmpeg")`
//! anywhere else, ever. In dev/CI ffmpeg is an external binary on PATH; in
//! shipped builds it is a bundled Tauri sidecar — this crate is the one place
//! that indirection lives.
//!
//! A render is three steps, in order:
//!
//! 1. [`plan`] reads the timeline and works out what occupies every frame of
//!    the requested range — no ffmpeg, no bytes, testable entirely on its own.
//! 2. [`pipe`] runs the two ffmpeg processes: one decoding a source into raw
//!    frames, one encoding raw frames into a file.
//! 3. [`run`] walks the plan, pulling frames from the first and handing them to
//!    the second, with compositing in between.
//!
//! That middle step is where Path B lives: ffmpeg decodes and encodes, and
//! every decision about what is on screen happens in our process. Compositing
//! is a passthrough for now — the seam is a single point in [`run`], and
//! Compositor v1 fills it in without either pipe changing shape.
//!
//! Boundary: no compositing logic (that is `scorsese-compositor`'s job — this
//! crate moves bytes, it never draws), no provider calls, no GUI. Depends on
//! `scorsese-core` and `scorsese-compositor`.

pub mod error;
pub mod frame;
pub mod pipe;
pub mod plan;
pub mod probe;
pub mod report;
pub mod run;
pub mod settings;
pub mod tools;

pub use error::{RenderError, Stage};
pub use frame::Frame;
pub use plan::{FrameRange, FrameRangeError, Plan, PlanError};
pub use probe::Ffprobe;
pub use report::{Note, RenderReport};
pub use run::Renderer;
pub use settings::{Bitrate, BitrateError, RenderSettings, Resolution, ResolutionError};
pub use tools::{Tools, ToolsError};
