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
//! A render is four steps, in order:
//!
//! 1. [`plan`] reads the timeline and works out what occupies every frame of
//!    the requested range — no ffmpeg, no bytes, testable entirely on its own.
//! 2. [`audio`] decodes and sums the audible clips into one finished mix, which
//!    the encoder then takes as an input file.
//! 3. [`pipe`] runs the ffmpeg processes: one decoding a source into raw frames
//!    or raw samples, one encoding them back into a file.
//! 4. [`run`] walks the plan, pulling frames from the first and handing them to
//!    the second, with compositing in between.
//!
//! The middle steps are where Path B lives: ffmpeg decodes and encodes, and
//! every decision about what is on screen — or in the mix — happens in our
//! process. Transforms, opacity, and layer order are `scorsese-compositor`'s,
//! called from [`run`]; summing samples is [`audio`]'s own.
//!
//! Boundary: no compositing logic (that is `scorsese-compositor`'s job — this
//! crate never draws), no provider calls, no GUI. Depends on `scorsese-core`
//! and `scorsese-compositor`.
//!
//! [`audio`] is the one thing here that processes rather than moves bytes, and
//! it sits in this crate because there is no `scorsese-mixer` to put it in. Its
//! arithmetic is deliberately free of ffmpeg and files, so making that crate —
//! the day a GUI wants to scrub audio without an encoder in the room — is a
//! file move rather than a rewrite.

pub mod audio;
pub mod error;
pub mod pipe;
pub mod plan;
pub mod probe;
pub mod properties;
pub mod raster;
pub mod report;
pub mod run;
pub mod settings;
pub mod text;
pub mod tools;

/// The frame buffer and raster types, which belong to the compositor — a frame
/// is what it produces. Re-exported so callers need not care which crate
/// defines them.
pub use scorsese_compositor::{Frame, PIXEL_FORMAT};

pub use audio::{Mix, Mixdown};
pub use error::{RenderError, Stage};
pub use plan::{FrameRange, FrameRangeError, Plan, PlanError};
pub use probe::{Ffprobe, fill_media};
pub use properties::{ANIMATABLE, Unknown, unknown_in};
pub use raster::Sizes;
pub use report::{Note, RenderReport};
pub use run::Renderer;
pub use settings::{
    Bitrate, BitrateError, RenderSettings, Resolution, ResolutionError, SampleRate, SampleRateError,
};
pub use text::Painter;
pub use tools::{Tools, ToolsError};
