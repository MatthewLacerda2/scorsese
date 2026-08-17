//! What a named symbol can be asked for that could never be drawn.
//!
//! Its own catalogue for [`super::ShapeProblem`]'s reason: these are findings
//! about the numbers *inside* one block rather than about whether the block is
//! there, and the assets catalogue is already where field presence is reported.
//!
//! **What is deliberately not here is the name.** Whether `clapperboard` is a
//! symbol this build ships is a fact about the catalogue, and the catalogue
//! lives in the compositor because a list of names is a set of property
//! *values*. Validation is about the document and stops at the edge of it — so
//! an unknown name is refused by `scorsese check` and by the render, exactly
//! where a `style`'s face is.

use crate::asset::AssetId;

/// One thing wrong with an `icon` asset's numbers.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum IconProblem {
    /// A symbol with no size. Zero, negative, or one of the values a float can
    /// hold that is not a number at all.
    #[error("asset `{asset}`: an icon is {size} of the frame's height, which has to be above zero")]
    NotSized {
        /// The icon asset.
        asset: AssetId,
        /// The fraction as written.
        size: f64,
    },

    /// A line with no thickness to draw it at.
    ///
    /// Refused rather than ignored, because an icon is *only* its line: unlike
    /// a shape, which can be a fill with no border, there is nothing left over
    /// to draw. A zero here is an empty layer, and an empty layer looks exactly
    /// like a layer that failed to render.
    #[error("asset `{asset}`: an icon's line is {width} of its own box, so nothing would be drawn")]
    NoThickness {
        /// The icon asset.
        asset: AssetId,
        /// The fraction as written.
        width: f64,
    },
}
