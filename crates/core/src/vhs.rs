//! The tape a picture was recorded onto, as five numbers and a mode.
//!
//! VHS is not one artefact, it is five that always arrive together: colour
//! smeared sideways because chroma was recorded at a fraction of luma's
//! bandwidth, snow on the luma, scanlines, a tracking wobble, and the torn band
//! at the bottom where the head switched. Nobody assembles that look from first
//! principles by accident, and nobody who *has* it wants one knob for the whole
//! of it either — the first shot where the wobble is too much is the shot where
//! a single number becomes unusable, because turning the wobble down turns the
//! look down with it.
//!
//! So this is one named effect with named sub-values, which is the middle of
//! those two shapes and deliberately so.
//!
//! **How that squares with the generality rule.** Core defines property types
//! and never property values, and a named cultural look is a value — which
//! argues for five loose primitives. The other rule argues the other way: a
//! person with an idea reaches for "make it look like VHS", not for "set the
//! chroma bandwidth to 0.3", and the scope rule cuts both ways — it is as much
//! a reason to build an obvious thing well as it is to refuse an elaborate one.
//! A named effect **with parameters** satisfies both, where a hardcoded look
//! would have satisfied neither: nothing here is a value somebody chose, every
//! field is a type with a neutral, and the look is what the numbers add up to.
//!
//! What the numbers *do* to pixels is `scorsese-compositor`'s to say, and it
//! says it next to the arithmetic. What lives here is the shape, the neutrals,
//! and the direction each one runs in.

use serde::{Deserialize, Serialize};

/// The tape treatment applied to a clip's pixels.
///
/// Every field is optional and every default is the **neutral** value, so a
/// `"vhs": {}` changes nothing and an absent `vhs` is the same as one whose
/// every field is left out.
///
/// The five numbers all run `0.0` (none) to `1.0` (the heaviest this offers),
/// and all five are animatable through the ordinary keyframe mechanism as
/// `vhs.chroma_bleed`, `vhs.noise` and so on — the same field-plus-track
/// bargain [`crate::Grade`] makes, where the field is the clip's baseline and a
/// track takes that one property over for the whole clip.
///
/// [`Vhs::mono`] is the exception and is a field only, for the reason
/// [`crate::Anchor`] is: it decides *what the other numbers mean* rather than
/// how much of them there is, and a mode ramping from one reading to another
/// over half a second is not a thing anybody means.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vhs {
    /// How far colour is smeared sideways, as a fraction of the layer's own
    /// width. `0.0` is none.
    ///
    /// The tape recorded chroma at a small fraction of luma's bandwidth, so
    /// colour arrives late and lags behind the edge it belongs to. The errors
    /// land on the colour-difference axes rather than on red, green and blue
    /// independently, which is where the familiar green and magenta of a bad
    /// tape comes from.
    ///
    /// Nothing at all when [`Vhs::mono`] is set, because then there is no
    /// chroma path to smear.
    #[serde(default)]
    pub chroma_bleed: f64,
    /// How much snow is laid over the picture. `0.0` is none.
    ///
    /// The same noise [`crate::Grade::grain`] draws — this is the tape's rather
    /// than the emulsion's, and a clip may carry both — so it moves with the
    /// frame, is strongest through the midtones, and is seeded from the clip
    /// and the frame and nothing else. In colour it speckles the
    /// colour-difference channels as well as the luma, which is what makes tape
    /// noise coloured where film grain is not.
    #[serde(default)]
    pub noise: f64,
    /// How dark the alternate lines are. `0.0` is none, `1.0` takes them most
    /// of the way to black.
    ///
    /// The line pitch is a fixed count down the picture rather than a count of
    /// pixels, for [`Vhs::chroma_bleed`]'s reason: a pattern measured in pixels
    /// would be invisible the moment the same edit was delivered at 4K.
    #[serde(default)]
    pub scanlines: f64,
    /// How far the tracking wobbles, as a fraction of the layer's own width.
    /// `0.0` holds still.
    ///
    /// A horizontal displacement that varies down the frame and changes every
    /// frame — a line read a little late, and how late is a fresh accident.
    /// **A fraction of the width and not the height**, unlike everything else
    /// this crate measures, because a line is displaced *along* itself and how
    /// far it can go is bounded by how long it is.
    #[serde(default)]
    pub jitter: f64,
    /// How torn the band at the bottom of the picture is. `0.0` leaves the
    /// bottom of frame alone.
    ///
    /// Head switching: the tape's two heads hand over a few lines before the
    /// bottom of the picture, and those lines are displaced sideways and lose
    /// their colour. One number sets both how tall the band is and how far it
    /// is torn, because a band nobody can see and a tear nobody can see are
    /// the same thing not happening.
    #[serde(default)]
    pub head_switch: f64,
    /// Whether the chroma path is modelled at all.
    ///
    /// **A mode with two honest positions, not a stylistic toggle**, and the
    /// difference is exactly this field's name. In colour there is a chroma
    /// path, so there is chroma to smear and chroma to speckle, and the result
    /// reads green and magenta. Without one the picture is grey and what is
    /// left is snow, scanlines and a wobble — which is a real artefact of its
    /// own, and a different one: early black-and-white tape, or a security
    /// camera, rather than a rented film.
    ///
    /// A field and not an animatable property — see the type's own doc.
    #[serde(default)]
    pub mono: bool,
}

impl Vhs {
    /// The tape that does nothing: no artefact, and the picture in colour.
    pub const NONE: Self = Self {
        chroma_bleed: 0.0,
        noise: 0.0,
        scanlines: 0.0,
        jitter: 0.0,
        head_switch: 0.0,
        mono: false,
    };

    /// True when this would leave every pixel exactly as it found it, so an
    /// untaped project costs nothing at all and a clip carrying `"vhs": {}` is
    /// byte-identical on screen to one carrying no `vhs` — which is what keeps
    /// every reference frame in the golden set meaning what it meant.
    ///
    /// **`mono` alone is not nothing.** Every number can be zero and the
    /// picture still change, because taking the colour out is a change; that is
    /// the one line here a reading of "all the numbers are zero" would get
    /// wrong.
    pub fn is_none(&self) -> bool {
        *self == Self::NONE
    }
}

impl Default for Vhs {
    fn default() -> Self {
        Self::NONE
    }
}
