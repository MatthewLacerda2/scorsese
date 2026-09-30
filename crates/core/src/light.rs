//! A layer's light of its own: the shadow it casts, the glow it gives off, and
//! how it lands on what is beneath it.
//!
//! Every other picture property in this model is about a layer's **own**
//! pixels — their colour, their sharpness, where they sit. These three are the
//! first about the layer's relation to the frame. A line, an icon or a caption
//! with no light of its own reads as flat vector art on a dark background; the
//! same element glowing reads as a lit display, and a cut-out picture with no
//! shadow looks pasted onto the frame until a soft contact shadow seats it.
//!
//! **All three are single-slider effects in every approachable editor** —
//! "shadow", "glow" and "blending mode" in Filmora and Canva — which is what
//! keeps them this side of the compositing-suite line. What is deliberately
//! absent is everything beyond that: inner glow, bevel and emboss, strokes as
//! an effect, "layer styles" as a family, the other dozen blend modes, and
//! anything that reads what is behind a layer beyond the blend itself.
//!
//! What the numbers *do* to pixels is `scorsese-compositor`'s to say, next to
//! the arithmetic — including where an animated value that has overshot its
//! range is clamped. What lives here is the shape, the defaults, and the
//! direction each number runs in.

use serde::{Deserialize, Serialize};

use crate::color::Rgba;

/// A drop shadow: the layer's own alpha, softened, tinted, offset, and drawn
/// **under** the layer.
///
/// Absent means no shadow, and absence is the only neutral a shadow has —
/// which is why a clip carries an `Option` of this, the way it does a
/// [`crate::ChromaKey`]. `"shadow": {}` is a shadow at the defaults below, so
/// asking for one without knowing four numbers still casts one.
///
/// **Every length here is a fraction of the layer's own height**, the unit
/// [`crate::Clip::blur`] uses — so the softness of a shadow and the blur of the
/// layer casting it agree, and the same numbers mean the same shadow at 1080p
/// and at 4K. The offset is measured against the height on **both** axes, so
/// `x` and `y` equal is a shadow falling at exactly 45° whatever the aspect of
/// the layer.
///
/// `opacity` is animatable as `shadow.opacity`, through the ordinary keyframe
/// mechanism with the ordinary meaning. The colour and the offset are not: a
/// colour is not a number, and an offset moving is a light moving, which is a
/// different effect from a shadow.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shadow {
    /// What colour the shadow is. Black unless said otherwise; its alpha
    /// multiplies [`Shadow::opacity`] rather than replacing it.
    #[serde(default = "default_color")]
    pub color: Rgba,
    /// How far right the shadow falls, as a fraction of the layer's own
    /// height. Negative is left.
    #[serde(default = "default_offset")]
    pub offset_x: f64,
    /// How far down the shadow falls, as a fraction of the layer's own height.
    /// Negative is up.
    #[serde(default = "default_offset")]
    pub offset_y: f64,
    /// How soft its edge is, measured exactly as [`crate::Clip::blur`] is: a
    /// fraction of the layer's own height, `0.0` a hard-edged silhouette.
    #[serde(default = "default_softness")]
    pub softness: f64,
    /// How dark it is: `0.0` no shadow at all, `1.0` the full colour wherever
    /// the layer is solid.
    #[serde(default = "default_opacity")]
    pub opacity: f64,
}

/// Black, for [`Shadow::color`].
fn default_color() -> Rgba {
    Rgba::BLACK
}

/// [`Shadow::OFFSET`], for serde.
fn default_offset() -> f64 {
    Shadow::OFFSET
}

/// [`Shadow::SOFTNESS`], for serde.
fn default_softness() -> f64 {
    Shadow::SOFTNESS
}

/// [`Shadow::OPACITY`], for serde.
fn default_opacity() -> f64 {
    Shadow::OPACITY
}

impl Shadow {
    /// How far a shadow falls, on each axis, when a document does not say —
    /// down and to the right, a light from the upper left, which is where
    /// nearly every interface and nearly every drawing puts it.
    pub const OFFSET: f64 = 0.01;

    /// How soft its edge is when a document does not say: soft enough to read
    /// as a shadow rather than a second, darker copy of the layer.
    pub const SOFTNESS: f64 = 0.02;

    /// How dark it is when a document does not say — half, which seats a
    /// layer without blacking out what the shadow falls on.
    pub const OPACITY: f64 = 0.5;
}

impl Default for Shadow {
    fn default() -> Self {
        Self {
            color: Rgba::BLACK,
            offset_x: Self::OFFSET,
            offset_y: Self::OFFSET,
            softness: Self::SOFTNESS,
            opacity: Self::OPACITY,
        }
    }
}

/// A soft halo of light around whatever the layer draws, centred on it and
/// drawn **under** it.
///
/// Absent means no glow, for the reason a [`Shadow`] is an `Option`.
/// `"glow": {}` is a glow at the defaults, in the layer's own colours.
///
/// `radius` and `intensity` are animatable as `glow.radius` and
/// `glow.intensity` — a pulse is two keyframes on the intensity. The colour is
/// not, for the reason a shadow's is not.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Glow {
    /// What colour the light is. **Absent means the layer's own colours**, so
    /// a cyan line glows cyan and a two-colour icon glows in both, which is
    /// what "make it glow" means before anybody has an opinion about colour.
    /// A colour here tints the whole halo, and its alpha multiplies the halo's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Rgba>,
    /// How far the halo reaches, measured exactly as [`crate::Clip::blur`] is:
    /// a fraction of the layer's own height.
    #[serde(default = "default_radius")]
    pub radius: f64,
    /// How bright the halo is. `0.0` is none, `1.0` the layer's own light
    /// spread out, and more than one is more light than the layer has — which
    /// is what a thin line needs, since spreading a two-pixel line over forty
    /// leaves very little of it anywhere.
    #[serde(default = "default_intensity")]
    pub intensity: f64,
}

/// [`Glow::RADIUS`], for serde.
fn default_radius() -> f64 {
    Glow::RADIUS
}

/// [`Glow::INTENSITY`], for serde.
fn default_intensity() -> f64 {
    Glow::INTENSITY
}

impl Glow {
    /// How far a glow reaches when a document does not say.
    pub const RADIUS: f64 = 0.02;

    /// How bright a glow is when a document does not say.
    pub const INTENSITY: f64 = 1.0;
}

impl Default for Glow {
    fn default() -> Self {
        Self {
            color: None,
            radius: Self::RADIUS,
            intensity: Self::INTENSITY,
        }
    }
}

/// How a layer lands on what is beneath it.
///
/// **Four, and the list is closed.** `normal` is the one every layer has
/// always had; `add` and `screen` are how light is layered, so overlapping
/// glowing dots brighten rather than hide each other; `multiply` is the
/// opposite, how a shadow or a stain darkens without covering. The other dozen
/// modes a compositing suite offers are each one nobody approachable reaches
/// for.
///
/// Static, not animatable: a mode is not a number, and one ramping into
/// another is not a thing anybody means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Blend {
    /// Covers what is beneath, in proportion to the layer's alpha.
    #[default]
    Normal,
    /// Adds the layer's light to what is beneath, clipping at white. Over
    /// black it is exactly `normal`.
    Add,
    /// Brightens what is beneath by the layer's light without ever passing
    /// white — a gentler `add`. Over black it is exactly `normal`.
    Screen,
    /// Darkens what is beneath by the layer's colour: white changes nothing,
    /// black makes black.
    Multiply,
}

impl Blend {
    /// Whether this is [`Blend::Normal`], so a clip can leave the field out.
    pub fn is_normal(&self) -> bool {
        *self == Self::Normal
    }

    /// The mode as the document spells it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Add => "add",
            Self::Screen => "screen",
            Self::Multiply => "multiply",
        }
    }
}
