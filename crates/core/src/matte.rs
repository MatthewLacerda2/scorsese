//! A track matte: a clip shown only where another clip is.
//!
//! Until this existed, a picture could only *arrive* — fade in, slide in,
//! scale up. It could not be **revealed**: wiped in from an edge, opened out of
//! a circle, seen through the letters of a title. Every approachable editor
//! has that as a mask ("rectangle, circle, text"), and every one of those
//! masks is a shape this model already draws.
//!
//! So a matte is **a reference to another clip**, not a second geometry. The
//! masked clip shows only where the matte clip's picture is opaque — or only
//! where it is not, [`Matte::invert`] — and whatever animates a clip animates
//! the mask: a wipe is a rectangle whose `transform.scale.x` runs `0 → 1`, an
//! iris is an ellipse growing, a soft edge is the matte clip's own `blur`. No
//! new property, no second shape language, and the groups, keyframes and
//! easings that already exist all apply to the mask for free.
//!
//! **Alpha only.** A luminance matte, a bezier mask, a per-vertex animated one
//! and anything tracked are the rotoscoping line the project refuses; the one
//! a person with an idea reaches for is "show this through that", and alpha is
//! all that takes.
//!
//! **Serving as a matte is implied by being named.** A clip that any clip names
//! as its matte is never drawn on its own track, whether or not the clip it
//! masks is on screen at that instant. A flag on the matte clip would be one
//! more thing to disagree with the reference — flagged but named by nobody,
//! named but unflagged — and neither disagreement has a good reading.
//!
//! What the matte *does* to pixels is `scorsese-compositor`'s to say, and
//! which references are allowed is validation's; this is the shape.

use serde::{Deserialize, Serialize};

use crate::timeline::ClipId;

/// Which clip masks this one, and which way round.
///
/// See the module doc for why a matte is a reference rather than a shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Matte {
    /// The clip whose picture is the mask, by id. It must be a visual clip on
    /// the same timeline as the one it masks — both on the project's own
    /// tracks, or both inside one group — must not be the masked clip itself,
    /// and must not carry a matte of its own.
    pub clip: ClipId,
    /// `false` — and absent — shows the masked clip only where the matte is
    /// opaque. `true` shows it only where the matte is **not**: a hole cut in
    /// the picture the shape of the matte.
    #[serde(default, skip_serializing_if = "is_false")]
    pub invert: bool,
}

impl Matte {
    /// A matte through `clip`, the ordinary way round.
    pub fn new(clip: ClipId) -> Self {
        Self {
            clip,
            invert: false,
        }
    }
}

/// Whether an `invert` is the default, so it can be left out of the document.
fn is_false(flag: &bool) -> bool {
    !*flag
}
