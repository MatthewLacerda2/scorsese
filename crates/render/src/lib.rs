//! # scorsese-render — ffmpeg orchestration
//!
//! Responsibility: everything that touches ffmpeg/ffprobe — probing imported
//! media, decoding sources to raw frames over pipes, piping composited raw
//! frames into an ffmpeg encode process, and render settings (resolution, fps,
//! bitrate, and the shape of the delivered file — user-chosen per render).
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
//! 3. `pipe` runs the ffmpeg processes: one decoding a source into raw frames
//!    or raw samples, one encoding them back into a file.
//! 4. [`Renderer`] walks the plan, pulling frames from the first and handing
//!    them to the second, with compositing in between.
//!
//! The middle steps are where Path B lives: ffmpeg decodes and encodes, and
//! every decision about what is on screen — or in the mix — happens in our
//! process. Transforms, opacity, and layer order are `scorsese-compositor`'s,
//! called from the [`Renderer`]; summing samples is [`audio`]'s own.
//!
//! Two more things exist so that a render can be *reviewed* without anyone
//! watching it, which is what an unattended agent has instead of eyes.
//! [`Description`] reads a plan back as prose — what is on screen when, at what
//! fit, with what animated, and what is audible under it — for nothing, since
//! the plan worked it all out already and used to throw it away. [`frames`]
//! pulls real frames out of a finished file as PNGs, which is the only half
//! that can catch the document being wrong about reality. They live here
//! because the plan is what knows the first and ffmpeg is what does the
//! second, and because the CLI, the MCP server and the golden harness all want
//! the same answer.
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
//!
//! ## What this publishes
//!
//! **A render, at the crate root.** [`Tools`] locates the binaries, [`Plan`]
//! decides what occupies every frame, [`Renderer`] produces the file and
//! [`RenderReport`] says what happened while it did; [`RenderSettings`] and
//! [`OutputFormat`] are what the caller chose. The review half is
//! [`Description`] — with [`Stretch`], [`Playing`] and [`Shown`] under it —
//! and [`Cue`], which names a frame in the delivered file. Media enters a
//! project through [`Ffprobe`] and [`fill_media`], and [`ANIMATABLE`] with
//! [`unknown_in`] is the vocabulary of animatable property paths a project is
//! checked against.
//!
//! **Five modules keep their path.** Three of them publish *verbs*, which need
//! their noun in front of them to read: [`mod@audio`] mixes
//! ([`audio::mixdown`]), measures ([`audio::measure`]) and owns the one
//! property name a volume keyframe spells ([`audio::path::VOLUME`]);
//! [`mod@frames`] pulls pictures back out of a finished file
//! ([`frames::extract`], [`frames::stills`], [`frames::read_png`],
//! [`frames::write_png`]); and [`mod@say`] turns a measurement into rows a
//! person reads ([`say::summary`], [`say::sections`], [`say::layers`],
//! [`say::survey`], [`say::comparison`]). [`mod@tools`] holds the two
//! environment variables that point the binaries somewhere else
//! ([`tools::FFMPEG_ENV`], [`tools::FFPROBE_ENV`]) beside the [`Tools`] the
//! root publishes. [`mod@plan`] is open because [`Segment`](plan::Segment) and
//! [`Shot`](plan::Shot) are what walking a plan means naming — its [`Plan`]
//! and [`FrameRange`] are reachable both ways, which is one path more than
//! anything needs and the next thing to tidy here.
//!
//! **Everything else is `pub(crate)`.** The ffmpeg processes themselves, the
//! rasters a fit is worked out on, the slug-card furniture, the font cache and
//! the whole of the encode side are steps a render is made of rather than
//! things to call: nothing above this crate composes them differently, and a
//! second caller of any of them would be a second renderer.

pub mod audio;
pub mod contact;
pub(crate) mod describe;
pub(crate) mod error;
pub(crate) mod format;
pub mod frames;
pub(crate) mod pipe;
pub mod plan;
pub(crate) mod probe;
pub(crate) mod properties;
pub(crate) mod raster;
pub(crate) mod report;
pub(crate) mod run;
pub mod say;
pub(crate) mod settings;
pub(crate) mod slug;
pub(crate) mod text;
pub mod tools;

/// The frame buffer and raster types, which belong to the compositor — a frame
/// is what it produces. Re-exported so callers need not care which crate
/// defines them.
pub use scorsese_compositor::{Frame, PIXEL_FORMAT};

/// Dissolving one shot into the next, which belongs to the compositor because
/// it names `opacity`. Re-exported for the same reason `audio::path::VOLUME`
/// is reachable from here: this is the seam where a caller that edits a
/// project meets the vocabulary of the thing that draws it.
pub use scorsese_compositor::{DissolveError, Placed, dissolve};

/// The coordinate ruler, which belongs to the compositor because it draws.
/// Re-exported because the callers that want it — the CLI and the MCP server —
/// reach a frame through this crate and never depend on the compositor
/// directly. [`grid::draw`] goes over a finished [`Frame`], once
/// [`Renderer::still`] has handed one back; a contact sheet rules its cells
/// inside [`contact::sheet`], where the source's own geometry still exists.
pub use scorsese_compositor::grid;

pub use contact::{ContactError, Look, Sheet};
pub use describe::{
    Animated, Commentary, Cue, CueError, Description, Moment, Playing, Shown, Stretch, Travel,
};
pub use error::{RenderError, Stage};
pub use format::{AudioCodec, Container, FormatError, OutputFormat, VideoCodec};
pub use plan::{FrameRange, FrameRangeError, Plan, PlanError, Showing};
pub use probe::{Ffprobe, fill_media};
pub use properties::{ANIMATABLE, Unknown, unknown_in};
pub use report::{Note, RenderReport, StandIn};
pub use run::Renderer;
pub use settings::{
    Bitrate, BitrateError, RenderSettings, Resolution, ResolutionError, SampleRate, SampleRateError,
};
pub use slug::{Absent, wording};
pub use tools::{Tools, ToolsError};
