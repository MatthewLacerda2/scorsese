//! Gradients: a [`Fill`] that is more than one colour, resolved to pixels.
//!
//! **Every coordinate a document writes is a fraction of a box**, and the box
//! is only known here, in pixels: a shape's own bounds, or the whole raster for
//! a `color` asset. So a gradient is resolved against the rectangle it paints
//! at the moment it is painted, and the same pill looks the same wherever on
//! the frame it sits.
//!
//! **Dithered, deliberately.** An 8-bit gradient across a large dark area has
//! only a few dozen levels to spend over a thousand pixels, so it comes out as
//! visible stripes — and H.264 makes them worse, because a flat band is exactly
//! what an encoder is best at keeping flat. tiny-skia's own gradient shaders do
//! not dither, which is why this evaluates the gradient itself, in floating
//! point, and adds noise before rounding: triangular, two levels either side
//! (σ ≈ 0.8 of a level, below what an eye resolves), the same value on all
//! three colour channels.
//!
//! **Noise, not an ordered pattern — measured, not assumed.** An 8×8 Bayer
//! matrix is the textbook dither and it does not survive an encoder: on a
//! vertical ramp every block along a row is identical, so x264 rounds them all
//! at the same row and the band edge comes back a straight line. Through a
//! 300 kbit/s encode of a 640×360 ramp eight levels deep, the worst jump
//! between neighbouring rows' mean luma was **2.0 levels undithered, 2.0 with
//! Bayer, 0.78 with noise one level either side and 0.37 with this** (frame 15
//! of 30); film grain on top (`grade.grain` 0.1) moved it only to 0.31, so the
//! dither is what does the work. `crates/render/tests/pipeline/banding.rs`
//! holds the steady state as a gate — 2.00 undithered, 0.28 dithered — and
//! the `gradients` golden, rendered at 60k, fails without it.
//!
//! **A hash, never a generator** — the `grain` module's, reused so the crate has
//! one answer to what a deterministic random number is. The noise is a pure
//! function of the pixel, so a gradient is the same picture on every render and
//! every machine, and a still layer's noise does not crawl from frame to frame.
//!
//! Colours are interpolated **premultiplied**, as CSS does, so a stop fading to
//! transparent does not drag a grey fringe through the middle of the ramp.

use scorsese_core::{Fill, Rgba, Stop};

use crate::frame::{BYTES_PER_PIXEL, Frame};
use crate::grain;

/// How far the dither may move a channel either side, in levels: the peak of
/// the triangular noise, which is two uniform values of this crate's hash
/// summed. Measured against one level either side in the module doc.
const DITHER: f64 = 2.0;

/// The seeds of the two noise fields the dither sums. Any two distinct
/// constants do; these spell what they are for.
const FIELDS: [u64; 2] = [0x6772_6164_6965_6e74, 0x6469_7468_6572_6564];

/// A rectangle of the raster, in pixels: left, top, width, height.
pub(crate) type Bounds = (f32, f32, f32, f32);

/// A gradient resolved against the box it paints, ready to be asked for the
/// colour of any pixel.
#[derive(Debug, Clone)]
pub(crate) struct Gradient {
    /// How far along the gradient a point is, before stops are consulted.
    axis: Axis,
    /// The stops, offsets as written, colours premultiplied — each channel on
    /// the byte scale, `0`–`255`, so a stop's own colour comes back exactly.
    stops: Vec<(f64, [f64; 4])>,
}

#[derive(Debug, Clone, Copy)]
enum Axis {
    /// `t = (p - start) · step`, where `step` is the unit direction divided by
    /// the gradient line's length.
    Linear { start: (f64, f64), step: (f64, f64) },
    /// `t = |p - center| / radius`.
    Radial { center: (f64, f64), radius: f64 },
}

impl Gradient {
    /// The gradient `fill` describes over `bounds`, or `None` when it is one
    /// colour — the caller already has a faster way to paint that — or when
    /// it has no stops or no extent to run over.
    pub(crate) fn new(fill: &Fill, (left, top, width, height): Bounds) -> Option<Self> {
        let (left, top) = (f64::from(left), f64::from(top));
        let (width, height) = (f64::from(width), f64::from(height));
        let axis = match fill {
            Fill::Solid(_) => return None,
            Fill::Linear(linear) => {
                let turn = linear.angle.to_radians();
                // CSS: 0° points up, clockwise. Screen y grows downwards.
                let (dx, dy) = (turn.sin(), -turn.cos());
                let length = (width * dx).abs() + (height * dy).abs();
                if !length.is_finite() || length <= 0.0 {
                    return None;
                }
                let middle = (left + width / 2.0, top + height / 2.0);
                Axis::Linear {
                    start: (middle.0 - dx * length / 2.0, middle.1 - dy * length / 2.0),
                    step: (dx / length, dy / length),
                }
            }
            Fill::Radial(radial) => {
                let radius = radial.radius * width.min(height);
                if !radius.is_finite() || radius <= 0.0 {
                    return None;
                }
                Axis::Radial {
                    center: (
                        left + radial.center.x * width,
                        top + radial.center.y * height,
                    ),
                    radius,
                }
            }
        };
        let stops: Vec<_> = fill.stops().iter().map(premultiplied).collect();
        (!stops.is_empty()).then_some(Self { axis, stops })
    }

    /// The colour of pixel `(x, y)` — sampled at its centre — as straight
    /// 8-bit RGBA, dithered.
    pub(crate) fn at(&self, x: u32, y: u32) -> [u8; 4] {
        let point = (f64::from(x) + 0.5, f64::from(y) + 0.5);
        let t = match self.axis {
            Axis::Linear { start, step } => {
                (point.0 - start.0) * step.0 + (point.1 - start.1) * step.1
            }
            Axis::Radial { center, radius } => {
                (point.0 - center.0).hypot(point.1 - center.1) / radius
            }
        };
        dithered(straight(self.sample(t)), noise(x, y))
    }

    /// The premultiplied colour `t` of the way along, clamped to the ends.
    ///
    /// Two stops at one offset are a hard edge: the later one wins from that
    /// offset on, as CSS draws it.
    fn sample(&self, t: f64) -> [f64; 4] {
        let first = self.stops[0];
        if t.is_nan() || t <= first.0 {
            return first.1;
        }
        for pair in self.stops.windows(2) {
            let ((from, low), (to, high)) = (pair[0], pair[1]);
            if t < to {
                let along = if to > from {
                    (t - from) / (to - from)
                } else {
                    1.0
                };
                return std::array::from_fn(|c| low[c] + (high[c] - low[c]) * along);
            }
        }
        self.stops[self.stops.len() - 1].1
    }
}

/// Paints the whole of `frame` with `fill`, replacing what was there — the
/// `color` asset's layer, whose box is the raster.
pub fn paint(frame: &mut Frame, fill: &Fill) {
    let resolution = frame.resolution();
    let (width, height) = (resolution.width(), resolution.height());
    let bounds = (0.0, 0.0, width as f32, height as f32);
    let Some(gradient) = Gradient::new(fill, bounds) else {
        frame.fill(fill.solid().unwrap_or(Rgba::WHITE));
        return;
    };
    let pixels = frame.bytes_mut().chunks_exact_mut(BYTES_PER_PIXEL);
    for (index, pixel) in (0u32..).zip(pixels) {
        pixel.copy_from_slice(&gradient.at(index % width, index / width));
    }
}

fn premultiplied(stop: &Stop) -> (f64, [f64; 4]) {
    let [r, g, b, a] = stop.color.channels().map(f64::from);
    let opacity = a / 255.0;
    (stop.at, [r * opacity, g * opacity, b * opacity, a])
}

/// Back to straight alpha. Fully transparent has no colour.
fn straight([r, g, b, a]: [f64; 4]) -> [f64; 4] {
    if a <= 0.0 {
        return [0.0; 4];
    }
    let opacity = a / 255.0;
    [r / opacity, g / opacity, b / opacity, a]
}

/// Each channel to a byte, with `noise` added before rounding to the nearest
/// level.
///
/// **A channel at either end of its range is left there.** Noise could only
/// push it one way — clamping takes the other half away — so dithering it
/// would be a bias rather than a dither; and a fill's opaque alpha must stay
/// opaque, or a solid gradient panel would come out faintly see-through.
fn dithered(color: [f64; 4], noise: f64) -> [u8; 4] {
    color.map(|channel| {
        if channel <= 0.0 || channel >= 255.0 {
            return channel.clamp(0.0, 255.0) as u8;
        }
        (channel + noise).round().clamp(0.0, 255.0) as u8
    })
}

/// The dither at pixel `(x, y)`: triangular, in `-DITHER..DITHER` levels.
fn noise(x: u32, y: u32) -> f64 {
    let at = (u64::from(y) << 32) | u64::from(x);
    let [first, second] = FIELDS.map(|field| grain::value(field, at));
    (first + second) * DITHER / 2.0
}

#[cfg(test)]
mod tests;
