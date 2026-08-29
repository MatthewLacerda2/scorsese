//! How a clip looks, as six numbers.
//!
//! A **grade** is the closed set of colour properties a clip can carry, and the
//! word "closed" is what keeps this inside the project's scope. scorsese is not
//! a compositing suite: there are no curves, no LUTs, no colour wheels, no
//! per-channel lift/gamma/gain. There is a knob and a slider, applied to a
//! clip, seen immediately — the Filmora 9 shape, which is what "make this look
//! older" reaches for.
//!
//! **Numbers only, never named looks.** The generality rule — core defines
//! property *types*, never property *values* — forbids a `"look": "70s"` field
//! here as surely as it forbids "make text red". `saturation: 0.18` is a
//! property; the 70s look is five values somebody chose, and those belong in a
//! project, a document, or an assistant's suggestion.
//!
//! **The test for belonging here is one pixel in, one pixel out.** Every field
//! below reads the pixel it is writing and nothing else, which is why `blur`
//! sits beside a grade rather than inside one — it reads a neighbourhood. The
//! test is about the *shape* of the operation and not about colour in the
//! narrow sense: `vignette` also consults where the pixel is, and `grain` also
//! consults which frame it is, and both still write one pixel from one pixel.
//!
//! What the numbers *do* to pixels is `scorsese-compositor`'s to say, and it
//! says it in one place, next to the arithmetic. What lives here is the shape,
//! the neutrals, and the direction each one runs in.

use serde::{Deserialize, Serialize};

/// The colour treatment applied to a clip's pixels before it is composited.
///
/// Every field is optional and every default is the **neutral** value, so a
/// grade that says nothing changes nothing and an absent grade is the same as
/// one whose every field is left out. That is what lets the field be added to
/// the format without touching a single existing document.
///
/// All six are animatable through the ordinary keyframe mechanism, as
/// `grade.saturation`, `grade.temperature` and so on. A grade written here is
/// the clip's **baseline**, and a keyframe track on one of those paths **takes
/// that property over outright** — a track always has a value, holding its
/// first and last, so it wins for the whole clip. The field goes on supplying
/// every property no track names, which is what makes "warm throughout, and
/// desaturate over the first second" two lines rather than a choice between
/// them.
///
/// Writing the number once is the common case and the reason this is a field at
/// all: three keyframe tracks holding one keyframe each is not how anybody
/// would choose to say "desaturate this shot".
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grade {
    /// How much colour, as a multiplier about the pixel's own grey.
    ///
    /// `1.0` leaves it alone, `0.0` is fully grey, and above `1.0`
    /// oversaturates. Negative values invert the colour around grey, which is
    /// not a thing anybody means, so they are clamped away where the pixels
    /// are.
    #[serde(default = "one")]
    pub saturation: f64,
    /// Which way the whites lean. `0.0` is untouched, negative is cooler
    /// (bluer), positive is warmer (more amber).
    ///
    /// Not a colour temperature in kelvin and deliberately not: a number a
    /// person turns until it looks right does not need a physical unit, and
    /// one that has a unit invites arithmetic nobody wanted to do.
    #[serde(default)]
    pub temperature: f64,
    /// How much light is added. `0.0` is untouched, negative darkens,
    /// positive lightens — an offset, so it lifts the blacks rather than
    /// multiplying what is already there.
    #[serde(default)]
    pub brightness: f64,
    /// How steep the range is, about mid-grey. `1.0` is untouched, below
    /// flattens toward grey, above steepens.
    ///
    /// One number on a slider, which is the whole of it. A curves editor is
    /// the thing this is deliberately not.
    #[serde(default = "one")]
    pub contrast: f64,
    /// How much the corners are darkened. `0.0` is none; higher darkens
    /// further, and `1.0` takes the corners to black.
    ///
    /// Measured from the **layer's** own centre rather than the frame's, so a
    /// vignette on a picture-in-picture darkens that picture's corners rather
    /// than a ring the layer happens to sit inside.
    #[serde(default)]
    pub vignette: f64,
    /// How much grain is laid over the picture. `0.0` is none — the clean,
    /// computed image — and `1.0` is the heaviest this offers.
    ///
    /// The one thing in this set that is **not a function of the pixel alone**:
    /// the noise moves with the frame, because grain that held still would read
    /// as dirt on the lens rather than as film. It still writes one pixel from
    /// one pixel, which is the test for belonging in a grade.
    ///
    /// **Nothing seeds it in the document, deliberately.** The noise field is
    /// derived from the clip's own id, so two clips of the same footage never
    /// carry the same grain, and the same project renders the same grain on
    /// every machine and in every run. A seed in the format would be a number
    /// nobody could choose meaningfully and everybody could copy by accident.
    #[serde(default)]
    pub grain: f64,
}

/// The multiplicative neutral, for the two fields whose is not zero.
fn one() -> f64 {
    1.0
}

impl Grade {
    /// The grade that changes nothing.
    pub const NEUTRAL: Self = Self {
        saturation: 1.0,
        temperature: 0.0,
        brightness: 0.0,
        contrast: 1.0,
        vignette: 0.0,
        grain: 0.0,
    };

    /// True when this grade would leave every pixel exactly as it found it.
    ///
    /// Asked before any pixel is touched, so an ungraded project costs nothing
    /// at all — and so that a clip carrying `"grade": {}` is byte-identical on
    /// screen to one carrying no grade, which is what keeps every reference
    /// frame in the golden set meaning what it meant.
    pub fn is_neutral(&self) -> bool {
        *self == Self::NEUTRAL
    }
}

impl Default for Grade {
    fn default() -> Self {
        Self::NEUTRAL
    }
}
