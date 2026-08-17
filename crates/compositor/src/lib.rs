//! # scorsese-compositor — frame rendering
//!
//! Responsibility: producing each output frame from decoded source frames —
//! transforms (position, scale, rotation, flip), alpha blending, opacity, and
//! later text and slug cards. CPU-first via tiny-skia; a wgpu backend arrives
//! later behind the same [`Compositor`] trait.
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
//! `transform.scale.x` mean anything is decided here, next to the code that
//! implements them, which is the generality rule holding in practice: a new
//! animatable property costs a change here and nothing to the format, the
//! model, or the renderer.
//!
//! That is also why the *list* of them lives here: [`ANIMATED`] publishes what
//! this compositor resolves, so something else can warn about a keyframe track
//! naming a property nobody animates. [`Registry`] is the machinery for asking;
//! the answer is never an error.
//!
//! [`Frame`] and [`Resolution`] live here too, because a frame buffer is this
//! crate's currency; `scorsese-render` re-exports them.
//!
//! [`text`] is the other thing that turns a model into pixels: a glyph
//! pipeline, and **the two font files this build ships**, since a system-font
//! lookup would render differently on every platform and the golden gate is
//! pixels. What it produces is an ordinary layer, so a title fades and moves
//! through the properties above rather than through anything of its own.
//!
//! [`mod@shape`] is the third: the boxes and ellipses a diagram is drawn from,
//! and the same bargain text makes — what comes out is an ordinary layer, so a
//! box that fades or slides is the existing properties acting on a layer that
//! happens to have a rectangle in it.
//!
//! [`mod@icon`] is the fourth, and it is the same argument the shapes won:
//! **the symbols this build ships**, vendored from Lucide and compiled in the
//! way the faces are. A named symbol drawn at the render's own resolution
//! rather than a PNG with alpha that goes soft the moment that resolution
//! changes, and one more layer once it is drawn.
//!
//! All of them, and the ruler, reach the raster through `paint`, which is the
//! crate's **one** rasteriser and one blend. Two would be two answers to what a
//! soft edge looks like, and pixels are what the golden gate compares.
//!
//! [`sheet`] tiles several frames into one labelled picture, so an assistant
//! can look at footage. It is the same two modules again — a label is a card,
//! and placing a cell is a layer — which is why it is forty lines of layout
//! rather than a path of its own.
//!
//! [`card`] is that same text over a panel of colour, which is all a slug card
//! is. It is a hundred lines that call the other two modules, and it is not a
//! rendering path of its own — that is the point of it.
//!
//! [`mod@grid`] is the one thing here that is part of no picture: a ruler drawn
//! *over* a finished frame, in the fractions `crop` and `transform.position`
//! are written in, so a coordinate can be read off a still instead of guessed
//! at. Nothing that renders a file calls it, and nothing it draws is ever
//! composited.
//!
//! Boundary: no ffmpeg invocation, no encoding, no file I/O on media, no
//! provider calls, no GUI event loop. It depends on `scorsese-core` for the
//! model and nothing above it. The shipped faces are compiled in with
//! `include_bytes!` rather than read at runtime, which is what keeps that
//! true; a project's own font arrives as bytes somebody else opened, and the
//! icon catalogue is compiled in the same way for the same reason.
//!
//! ## What this publishes
//!
//! At the crate root, the compositing vocabulary: the [`Compositor`] trait and
//! the [`CpuCompositor`] behind it, the [`Layer`] they take and the [`Frame`],
//! [`Resolution`] and [`PIXEL_FORMAT`] a picture is carried in, the
//! [`Properties`] one instant of a clip resolves to, the [`ANIMATED`] list with
//! the [`Registry`] that searches it, and the two fades ([`fade_in`],
//! [`fade_out`]).
//!
//! [`text`], [`card`], [`mod@shape`], [`mod@icon`], [`mod@grid`] and
//! [`mod@dissolve`] keep
//! their module path as well, because what they publish are *verbs* — `draw`,
//! `draw_in`, `draw_line`, `dissolve` — and a verb that general needs the noun
//! in front of it to read. `text::draw`, `card::draw` and `shape::draw` could
//! not all sit at the root in any case.
//! The face those verbs set words in is [`text::Font`], and that is its only
//! spelling: the root re-export it also had was a second name for a type every
//! caller already reached through `text`.
//!
//! Everything else is `pub(crate)`. How a face is read, how a line is broken,
//! how glyphs are shaped and filled, how one layer is blended onto another are
//! this crate's own business: `scorsese-render`, its only dependent, asks it
//! for a picture and never for the steps that made one.

mod area;
mod blur;
pub mod card;
mod compose;
mod cpu;
pub mod dissolve;
mod frame;
mod grade;
pub mod grid;
pub mod icon;
mod paint;
mod properties;
mod registry;
pub mod shape;
pub mod sheet;
pub mod text;
pub mod waveform;

pub use area::{Area, on_canvas};
pub use compose::{CompositeError, Compositor, Layer};
pub use cpu::CpuCompositor;
pub use dissolve::{DissolveError, Placed, dissolve};
pub use frame::{BYTES_PER_PIXEL, Frame, PIXEL_FORMAT, Resolution, ResolutionError};
pub use properties::{ANIMATED, Properties, fade_in, fade_out, path};
pub use registry::{Property, Registry};
