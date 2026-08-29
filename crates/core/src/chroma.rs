//! Which of a clip's pixels are not there: the chroma key.
//!
//! Every other picture property in this model answers *what colour is this
//! pixel*. This one answers whether there is a pixel at all — it is the only
//! thing in scorsese that can make a source's own pixels transparent, and
//! before it the only alpha in the system was the alpha a source arrived with.
//!
//! **Four values, and that is the whole feature**: the screen's colour, how far
//! from it still counts as screen, how wide the ramp out of that is, and
//! whether the colour the screen bounced back onto the subject is pulled out
//! again. Garbage mattes, animated masks, light wrap, per-channel despill and
//! combined luma-plus-chroma mattes are a compositing suite's answers to a
//! question nobody editing a video asks.
//!
//! **There is no luma key and there will not be one**, which is worth recording
//! because it will be asked again: knocking out a white or a black background
//! also knocks out every white and black *in the picture* — eyes, teeth, the
//! shine on hair. The answer to "I want a cutout" is to shoot or generate
//! against a saturated colour, not to make the keyer cleverer. It is also why
//! [`ChromaKey::is_keyable`] exists: a key on a grey is a luma key wearing a
//! colour's clothes, and validation refuses it.
//!
//! **Beside [`crate::Grade`] rather than inside it.** A grade is the closed set
//! of properties that read one pixel and write one, and a key passes that test
//! exactly — read a pixel, decide its alpha. What keeps it out of the struct is
//! the other half of that doc: a grade is the closed set of *colour*
//! properties, and every field in it is a number with a neutral. A key is a
//! colour plus three settings and its neutral is *absence*, so filing it under
//! `Grade` would make that doc untrue about what it holds.
//!
//! What the four values *do* to pixels is `scorsese-compositor`'s to say, next
//! to the arithmetic, exactly as it is for a grade. What lives here is the
//! shape, the defaults, and the direction each one runs in.

use serde::{Deserialize, Serialize};

use crate::color::Rgba;

/// How far apart a key colour's strongest and weakest channels must be, in
/// 8-bit levels, before it counts as a colour rather than a shade of grey.
///
/// Sixteen out of 255 is deliberately low: this is not a judgement about
/// whether a key will work well, which is the author's to make, but the line
/// under which it is not a chroma key at all. A neutral has no hue for a
/// distance to be measured from, so a key on one would separate pixels by how
/// *bright* they are — the luma key this model does not have.
pub const MIN_KEY_SPREAD: u8 = 16;

/// The screen colour a clip is keyed against, and the three settings that say
/// how forgiving the key is.
///
/// Absent means no key, and absence is the only neutral there is: a key needs a
/// colour, and there is no colour that means "do not key". That is why this is
/// an `Option` on a clip where a [`crate::Grade`] is a struct with a neutral.
///
/// `tolerance` and `softness` are animatable as `chroma_key.tolerance` and
/// `chroma_key.softness`, through the ordinary keyframe mechanism and with the
/// ordinary meaning: the field is the clip's baseline and a track takes that
/// property over for the whole clip. `color` and `spill` are not — a colour is
/// not a number, and a boolean ramped halfway is not a state anything is in.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChromaKey {
    /// The screen's colour — what the camera saw where the subject was not.
    ///
    /// Sampled from the footage rather than guessed at: a lit green screen is
    /// never `#00ff00`, and a key aimed at a colour nothing in the shot
    /// actually is has to be given a tolerance wide enough to swallow the
    /// subject before it swallows the screen.
    ///
    /// **Its alpha is ignored.** A key names a colour the camera recorded, and
    /// a recorded colour is opaque; the notation carries an alpha because one
    /// notation reads every colour in the document.
    pub color: Rgba,
    /// How far from [`ChromaKey::color`] a pixel may be and still be screen.
    /// `0.0` keys only an exact match; higher takes more of the shot with it.
    ///
    /// A distance in the plane the compositor measures colour in, so its scale
    /// is that plane's — see `scorsese-compositor`'s `chroma` module, and
    /// `docs/project-format.md` for the numbers an author needs.
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,
    /// How wide the ramp from screen to subject is, in the same units as
    /// [`ChromaKey::tolerance`] and measured outward from it.
    ///
    /// `0.0` is a hard cutout, which reads as a paper doll on anything with a
    /// soft edge; a little is what makes hair and motion blur survive. Below
    /// the tolerance every pixel is fully gone, above tolerance plus this every
    /// pixel is fully kept, and between them the alpha ramps — the two
    /// thresholds every keyer has.
    #[serde(default = "default_softness")]
    pub softness: f64,
    /// Whether the colour the screen bounced onto the subject is pulled back
    /// out of it.
    ///
    /// **One boolean**, deliberately. A strength, a radius or per-channel
    /// controls are the compositing suite arriving one field at a time; this is
    /// the single thing that stops a key reading as amateur, which is the green
    /// rim on every soft edge and every strand of hair.
    ///
    /// It costs something and the cost is worth stating: the suppression is
    /// about the *hue* that was keyed, so it pulls that hue out of the whole
    /// layer — a genuinely green jacket in front of a green screen comes back
    /// less green. That is the price of one boolean, and turning it off is the
    /// escape hatch.
    #[serde(default)]
    pub spill: bool,
}

/// A forgiving-but-not-reckless default, in the units
/// [`ChromaKey::tolerance`] is measured in.
fn default_tolerance() -> f64 {
    ChromaKey::TOLERANCE
}

/// Enough ramp that an edge is not a cutout, and not so much that it eats the
/// subject.
fn default_softness() -> f64 {
    ChromaKey::SOFTNESS
}

impl ChromaKey {
    /// What `tolerance` is when a document does not say.
    ///
    /// Chosen so that `{ "color": "#00b140" }` — a key with nothing but a
    /// screen colour on it — keys the screen rather than doing nothing at all.
    /// A neutral default would be honest about saying nothing and useless: the
    /// field exists to key, and a key that keys nothing is a field somebody has
    /// to discover a second number before they can use.
    pub const TOLERANCE: f64 = 0.25;

    /// What `softness` is when a document does not say — a narrow ramp, so an
    /// edge is soft rather than cut out, without reaching far enough into the
    /// subject to be noticed.
    pub const SOFTNESS: f64 = 0.1;

    /// A key on `color`, at the defaults.
    pub fn new(color: Rgba) -> Self {
        Self {
            color,
            tolerance: Self::TOLERANCE,
            softness: Self::SOFTNESS,
            spill: false,
        }
    }

    /// True when [`ChromaKey::color`] is a colour something could be keyed
    /// against: its strongest and weakest channels differ by at least
    /// [`MIN_KEY_SPREAD`].
    ///
    /// The question is whether the colour has a *hue*, not whether it is a good
    /// screen. Black, white and every grey between them fail it, and so does
    /// the whole of what a luma key would be aimed at.
    pub fn is_keyable(&self) -> bool {
        let (r, g, b) = (self.color.r, self.color.g, self.color.b);
        r.max(g).max(b) - r.min(g).min(b) >= MIN_KEY_SPREAD
    }
}
