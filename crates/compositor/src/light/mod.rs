//! A layer's light of its own: the shadow it casts and the glow it gives off,
//! grown from its alpha, and the blend it lands on the canvas with.
//!
//! **Both are the layer's alpha, blurred, tinted, and drawn under it** — the
//! shadow offset, the glow centred. The blur is [`crate::blur`]'s kernel and
//! not a second one, so a shadow's `softness`, a glow's `radius` and a layer's
//! own `blur` are one measurement: a fraction of the layer's own height, three
//! box passes deep.
//!
//! **This is the last stage a layer's own pixels go through**, after the key,
//! the grade, the blur, the aberration and the tape. It acts on the finished
//! picture of the thing, so a blurred layer casts a blurred shadow and a keyed
//! one casts the shadow of what the key left. On a group clip that finished
//! picture is the whole group — [`crate::Compositor::offscreen`]'s canvas — so
//! one glow lights every member, as one diagram.
//!
//! **The picture comes out bigger than it went in.** A halo reaches past the
//! layer's own edges — three radii for a glow, three plus the offset for a
//! shadow — and a layer whose raster is smaller than the canvas (a cut-out
//! picture at its native size, a letterboxed plate) would otherwise have it
//! cut off square at its old box. So the result is padded by exactly that
//! reach on every side, and [`Lit::pad`] says by how much so the caller can
//! place the padded picture where the unpadded one would have been. Every drawn
//! layer — a title, a shape, an icon, a group — is already the size of the
//! render's raster and loses nothing either way; it is footage and stills this
//! is for.
//!
//! **Shadow, then glow, then the layer, all source-over, and only then does the
//! result meet the canvas** — once, with the layer's opacity and its blend. So
//! the three are one picture: a layer fading out takes its shadow with it
//! without the shadow showing through the layer, and an `add` layer adds its
//! glow as light too. That is also why a shadow on an `add` or a `screen`
//! layer is invisible over black, and should be: dark light is no light.
//!
//! Premultiplied throughout, like everything after the premultiply in
//! [`crate::CpuCompositor`]: the blur needs it, and a transparent pixel has to
//! contribute nothing rather than whatever colour it was stored as.

mod mix;

use scorsese_core::{Blend, Glow, Shadow};
use tiny_skia::BlendMode;

use crate::blur;
use crate::frame::{BYTES_PER_PIXEL, Resolution};

/// The brightest a glow is drawn: its intensity is clamped to `0.0` – this
/// where it is drawn, whatever an overshooting easing asked for.
///
/// Four, because a glow's use for more than one is thin lines — spreading a
/// line two pixels wide over forty leaves a halo a twentieth as dense, and it
/// takes a gain to bring it back — and past four every halo that has anything
/// in it has already saturated into a solid block, which is a different look.
pub const MAX_GLOW_INTENSITY: f64 = 4.0;

/// How far a shadow may be offset, as a fraction of the layer's height: one
/// whole height. Past that the shadow is not the layer's any more, and the
/// padding it costs grows without anybody seeing more of it.
const MAX_OFFSET: f64 = 1.0;

/// The canvas blend a layer's [`Blend`] names.
///
/// Four of tiny-skia's modes, and the mapping is the whole of the feature: the
/// modes are the textbook ones on premultiplied colour, and `add` is what
/// tiny-skia calls `Plus`. On an opaque canvas every one of them leaves it
/// opaque, so the render's canvas never needs demultiplying.
pub(crate) fn blend_mode(blend: Blend) -> BlendMode {
    match blend {
        Blend::Normal => BlendMode::SourceOver,
        Blend::Add => BlendMode::Plus,
        Blend::Screen => BlendMode::Screen,
        Blend::Multiply => BlendMode::Multiply,
    }
}

/// The copies a lit layer needs, kept between frames for the reason every
/// other stage's scratch is.
#[derive(Debug, Default)]
pub(crate) struct Buffers {
    /// The layer, padded with transparency out to its halo's reach.
    padded: Vec<u8>,
    /// The finished picture: shadow, glow and layer, one over the next.
    lit: Vec<u8>,
    /// The blur's own pair.
    blurred: blur::Buffers,
}

/// A layer's finished picture, and how much bigger than the layer it is.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Lit<'a> {
    /// Premultiplied RGBA.
    pub(crate) bytes: &'a [u8],
    /// The size of `bytes`, which is the layer's own plus the padding.
    pub(crate) resolution: Resolution,
    /// Pixels of padding on the left and right, then on the top and bottom —
    /// what the caller shifts the picture back by so the layer lands where it
    /// would have unlit.
    pub(crate) pad: (u32, u32),
}

/// A shadow, resolved to this layer's pixels.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Cast {
    /// Its colour, straight.
    pub(crate) rgb: [u8; 3],
    /// How much of that colour a fully solid pixel of the layer casts:
    /// opacity times the colour's own alpha, `0.0` to `1.0`.
    pub(crate) strength: f32,
    /// Where it falls, in whole pixels of the layer.
    pub(crate) offset: (i64, i64),
    /// How soft, in the blur's pixels.
    pub(crate) radius: usize,
}

/// A glow, resolved to this layer's pixels.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Halo {
    /// A colour to tint it, straight, or the layer's own when `None`.
    pub(crate) tint: Option<[u8; 4]>,
    /// The clamped intensity.
    pub(crate) gain: f32,
    /// How far it reaches, in the blur's pixels.
    pub(crate) radius: usize,
}

/// A number from a document or a track, clamped to `0.0` – `max`, with a
/// non-number as nothing — the clamp the property docs promise.
fn within(value: f64, max: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, max)
    }
}

/// The shadow in this layer's pixels, or `None` when it would draw nothing.
pub(crate) fn cast(shadow: Option<Shadow>, height: u32) -> Option<Cast> {
    let shadow = shadow?;
    let strength = within(shadow.opacity, 1.0) * f64::from(shadow.color.a) / 255.0;
    if strength <= 0.0 {
        return None;
    }
    let pixels = |offset: f64| {
        let offset = if offset.is_nan() { 0.0 } else { offset };
        (offset.clamp(-MAX_OFFSET, MAX_OFFSET) * f64::from(height)).round() as i64
    };
    Some(Cast {
        rgb: [shadow.color.r, shadow.color.g, shadow.color.b],
        strength: strength as f32,
        offset: (pixels(shadow.offset_x), pixels(shadow.offset_y)),
        radius: blur::radius(shadow.softness, height),
    })
}

/// The glow in this layer's pixels, or `None` when it would draw nothing.
pub(crate) fn halo(glow: Option<Glow>, height: u32) -> Option<Halo> {
    let glow = glow?;
    let gain = within(glow.intensity, MAX_GLOW_INTENSITY);
    if gain <= 0.0 || glow.color.is_some_and(|color| color.a == 0) {
        return None;
    }
    Some(Halo {
        tint: glow.color.map(scorsese_core::Rgba::channels),
        gain: gain as f32,
        radius: blur::radius(glow.radius, height),
    })
}

/// How far past the layer's edges its light reaches, across and down.
///
/// Three radii, because three box passes of radius `r` reach exactly `3r` and
/// not a pixel further — [`crate::blur`]'s own tests pin that.
fn reach(cast: Option<Cast>, halo: Option<Halo>) -> (u32, u32) {
    let shadow = cast.map_or((0, 0), |cast| {
        let soft = 3 * cast.radius as u64;
        (
            soft + cast.offset.0.unsigned_abs(),
            soft + cast.offset.1.unsigned_abs(),
        )
    });
    let glow = halo.map_or(0, |halo| 3 * halo.radius as u64);
    (shadow.0.max(glow) as u32, shadow.1.max(glow) as u32)
}

/// The layer's finished picture: its shadow and its glow drawn under it, on a
/// raster padded out to their reach.
///
/// Hands `source` back untouched, with no padding, when there is no light to
/// draw — which is almost every layer — and when its length disagrees with the
/// resolution it claims, which is refused a few lines later where every other
/// malformed layer is.
pub(crate) fn into<'a>(
    buffers: &'a mut Buffers,
    source: &'a [u8],
    resolution: Resolution,
    cast: Option<Cast>,
    halo: Option<Halo>,
) -> Lit<'a> {
    let untouched = Lit {
        bytes: source,
        resolution,
        pad: (0, 0),
    };
    if (cast.is_none() && halo.is_none()) || source.len() != resolution.pixels() * BYTES_PER_PIXEL {
        return untouched;
    }
    let pad = reach(cast, halo);
    let Ok(padded_resolution) = Resolution::source(
        resolution.width() + 2 * pad.0,
        resolution.height() + 2 * pad.1,
    ) else {
        return untouched;
    };
    let Buffers {
        padded,
        lit,
        blurred,
    } = buffers;
    mix::pad(padded, source, resolution, pad);
    lit.clear();
    lit.resize(padded.len(), 0);
    if let Some(cast) = cast {
        let softened = blur::into(blurred, padded, padded_resolution, cast.radius);
        mix::shadow(lit, softened, padded_resolution, cast);
    }
    if let Some(halo) = halo {
        let spread = blur::into(blurred, padded, padded_resolution, halo.radius);
        mix::glow(lit, spread, halo);
    }
    mix::over(lit, padded);
    Lit {
        bytes: lit,
        resolution: padded_resolution,
        pad,
    }
}
