//! # scorsese-compositor — frame rendering
//!
//! Responsibility: producing each output frame from decoded source frames —
//! transforms (position/scale), alpha blending, opacity, and later text and
//! slug cards. CPU-first via tiny-skia; a wgpu backend arrives later behind the
//! same [`Compositor`] trait.
//!
//! Compositing is ours; ffmpeg only decodes and encodes (Path B). This crate
//! consumes raw frames plus already-evaluated property values and emits raw
//! frames. It animates nothing itself: a [`Layer`] describes one instant, and
//! working out what that instant looks like is [`Properties::at`]'s job, from
//! the generic keyframe tracks `scorsese-core` holds.
//!
//! **This crate is where property paths acquire meaning.** `scorsese-core` knows
//! only that a keyframe track animates *some* numeric property — the mechanism
//! there is deliberately ignorant of which. That `opacity` and
//! `transform.scale.x` mean anything is decided in [`properties`], next to the
//! code that implements them, which is the generality rule holding in practice:
//! a new animatable property costs a change here and nothing to the format, the
//! model, or the renderer.
//!
//! That is also why the *list* of them lives here: [`properties::ANIMATED`]
//! publishes what this compositor resolves, so something else can warn about a
//! keyframe track naming a property nobody animates. [`registry`] is the
//! machinery for asking; the answer is never an error.
//!
//! [`Frame`] and [`Resolution`] live here too, because a frame buffer is this
//! crate's currency; `scorsese-render` re-exports them.
//!
//! Boundary: no ffmpeg invocation, no encoding, no file I/O on media, no
//! provider calls, no GUI event loop. It depends on `scorsese-core` for the
//! model and nothing above it.

pub mod compose;
pub mod cpu;
pub mod frame;
pub mod properties;
pub mod registry;

pub use compose::{CompositeError, Compositor, Layer};
pub use cpu::CpuCompositor;
pub use frame::{BYTES_PER_PIXEL, Frame, PIXEL_FORMAT, Resolution, ResolutionError};
pub use properties::{ANIMATED, Properties, fade_in, fade_out, path};
pub use registry::{Property, Registry};
