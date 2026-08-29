//! The noise a photochemical picture has and a computed one never does.
//!
//! A grade can make a picture warm, flat, dark, soft and falling off at the
//! corners and it still reads as computed, because every one of those knobs
//! leaves the picture *clean*. Grain is what film has instead: a texture that
//! belongs to the emulsion rather than to the scene, and the oldest trick there
//! is for making shots that were made separately sit together.
//!
//! **A hash, never a generator.** Every value here is a pure function of the
//! clip, the frame, and the pixel — nothing is carried between pixels, between
//! frames, or between renders. That is not a performance choice: golden renders
//! compare frames, and grain drawn from a random number generator would make
//! every render a different picture, which does not fail the pixel gate loudly,
//! it makes it meaningless. It also means a frame can be drawn by any worker in
//! any order, which is what the render pipeline actually does.
//!
//! **The arithmetic is integer until the last step, and then only IEEE.** No
//! `sin`, no `exp`, no library transcendental anywhere — those are the
//! functions whose last bit differs between platforms, and a grain that differs
//! in its last bit is a golden render that fails on a machine that is not this
//! one. What is left is wrapping multiplies, shifts, and a division by a
//! constant, all of which are exactly specified.
//!
//! Reused rather than reimplemented: anything else wanting film-like noise —
//! a composite retro effect, say — builds a [`Grain`] and calls
//! [`Grain::at`] rather than growing a second noise function that drifts from
//! this one.

use scorsese_core::Frames;

use crate::frame::Resolution;

/// How far a full-strength grain moves a mid-grey pixel, as a fraction of the
/// displayable range.
///
/// `1.0` is meant to be the heaviest grain anybody would ask for rather than
/// the heaviest the arithmetic can express, so it lands where a pushed 16mm
/// stock does and not where a broken sensor does. Ordinary use is a tenth of
/// it.
const SWING: f64 = 0.25;

/// The raster height at which one grain cell is one pixel.
///
/// Grain has a **size**, and it is a size on the film rather than a count of
/// pixels — so the same number has to read as the same texture whether the
/// project is delivered at 1080p or at 4K. Without this, a 4K delivery of the
/// same edit comes out visibly finer-grained than the 1080p one, which is the
/// same mistake a blur measured in pixels would make. Measured against the
/// **layer's** own height, like [`crate::blur`], so a layer scaled up on the
/// canvas has its grain scaled up with it.
///
/// Below this height a cell is one pixel, because there is nothing finer for it
/// to be.
const REFERENCE_HEIGHT: u32 = 1080;

/// Two odd multipliers, to spread a pixel's coordinates across the whole word
/// before [`mix`] avalanches them. The first is the golden ratio's reciprocal
/// scaled to 64 bits; the second is xxhash's second prime. Any two large odd
/// constants would do — these are simply ones known to be good at it.
const ACROSS: u64 = 0x9E37_79B9_7F4A_7C15;
const DOWN: u64 = 0xC2B2_AE3D_27D4_EB4F;

/// splitmix64's finaliser: 64 bits in, 64 well-mixed bits out.
///
/// Chosen because it is short, has no state, and is specified entirely in
/// wrapping integer arithmetic — so it produces the same bits on every target,
/// which is the whole requirement.
const fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Where a clip's noise field starts at one instant.
///
/// **Seeded from the clip's id and the frame, and from nothing else.** From the
/// id, so two clips of the same footage never carry the same grain — the
/// mistake somebody would notice. From the frame, so the grain crawls the way
/// film does instead of sitting still like dirt on the lens. From nothing else,
/// so the value for a frame never depends on a frame drawn before it, and the
/// same project renders the same grain on any machine.
///
/// The time is the clip's own elapsed frame rather than the timeline's, for the
/// reason keyframes are: moving a clip along the timeline must not rewrite what
/// it looks like.
pub(crate) fn seed(clip: &str, t: Frames) -> u64 {
    // FNV-1a over the id's bytes. A short, exactly specified string hash, which
    // is all this needs — the result goes straight through `mix` below.
    const OFFSET_BASIS: u64 = 0xCBF2_9CE4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01B3;
    let mut hash = OFFSET_BASIS;
    for byte in clip.as_bytes() {
        hash = (hash ^ u64::from(*byte)).wrapping_mul(PRIME);
    }
    mix(hash ^ mix(t.get()))
}

/// One layer's grain, resolved for one frame.
///
/// Built once per layer and asked per pixel, so everything that does not vary
/// across the raster — the strength, the cell size, the frame's seed — is
/// worked out here rather than in the inner loop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Grain {
    seed: u64,
    /// The peak deviation at mid-grey, in the same `0.0..=1.0` units the grade
    /// works in.
    strength: f64,
    /// The raster's width, which is what turns a pixel index into a coordinate.
    width: u64,
    /// How many pixels across one cell of noise is. Never zero.
    cell: u64,
}

impl Grain {
    /// The grain of a layer this size at this strength, or `None` when there is
    /// none — so a grade with the field left alone costs the inner loop
    /// nothing at all.
    ///
    /// Clamped to `0.0..=1.0` the way [`scorsese_core::Grade::vignette`] is: a
    /// negative amount is not grain in the other direction, there being no such
    /// thing, and past `1.0` is past the point where more is a look anybody
    /// asked for.
    pub(crate) fn new(seed: u64, amount: f64, resolution: Resolution) -> Option<Self> {
        let amount = amount.clamp(0.0, 1.0);
        if amount <= 0.0 {
            return None;
        }
        Some(Self {
            seed,
            strength: amount * SWING,
            width: u64::from(resolution.width()),
            cell: u64::from((resolution.height() / REFERENCE_HEIGHT).max(1)),
        })
    }

    /// The signed offset every channel of one pixel gets, in `0.0..=1.0` units.
    ///
    /// **Monochrome**, the same value on all three channels: silver halide
    /// crystals are not coloured, and noise drawn independently per channel is
    /// the speckle of a video sensor rather than the texture of film. Only one
    /// of the two is what a retro look is reaching for, and it is this one.
    ///
    /// **Weighted by the pixel's own luma**, peaking at mid-grey and falling to
    /// nothing at both ends of the range. That is where grain actually lives:
    /// uniform noise across the whole range is precisely what reads as digital
    /// noise added in post, because it puts texture into blown highlights and
    /// crushed blacks, where a photochemical image has none.
    ///
    /// `index` is the pixel's offset into the layer's own raster, so the grain
    /// is fixed to the layer's pixels and travels with it — like the vignette,
    /// which is measured from the layer's own centre.
    pub(crate) fn at(&self, index: usize, luma: f64) -> f64 {
        let index = index as u64;
        let (x, y) = (
            index % self.width / self.cell,
            index / self.width / self.cell,
        );
        let noise = mix(self.seed ^ x.wrapping_mul(ACROSS) ^ y.wrapping_mul(DOWN));
        self.strength * midtones(luma) * signed(noise)
    }
}

/// How much of the grain a pixel of this brightness gets: `1.0` at mid-grey,
/// nothing at black or white, by `1 − (2l − 1)²`.
///
/// A parabola rather than anything shaped more carefully, because the shape
/// that matters is "most in the middle, none at the ends" and every curve with
/// that shape looks the same once it is scaled to a few levels out of 255.
fn midtones(luma: f64) -> f64 {
    let from_mid = 2.0 * luma.clamp(0.0, 1.0) - 1.0;
    1.0 - from_mid * from_mid
}

/// Well-mixed bits as a number in `-1.0..=1.0`, uniformly.
///
/// The top 32 bits rather than the whole word: a `u32` converts to `f64`
/// exactly, so this is one exactly-rounded division and two exact operations,
/// and no platform has room to disagree about any of them.
fn signed(bits: u64) -> f64 {
    f64::from((bits >> 32) as u32) / f64::from(u32::MAX) * 2.0 - 1.0
}
