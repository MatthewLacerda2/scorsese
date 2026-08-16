//! What a drawn shape can be asked for that could never be drawn.
//!
//! Its own catalogue for [`super::VideoProblem`]'s reason: these are findings
//! about the numbers *inside* one block rather than about whether the block is
//! there, and the assets catalogue is already the place where field presence is
//! reported. Keeping them apart is what lets each grow on its own.
//!
//! Every one of them is answerable from the document alone — no raster, no
//! media, no measurement. That is deliberate, and it is why `radius` is a
//! fraction of the shape rather than of the frame: a rounding larger than the
//! box it rounds could not be caught here at all if it were written in the
//! frame's units, and would surface as a picture that looked wrong instead.

use crate::asset::AssetId;

/// One thing wrong with a `shape` asset's geometry or colouring.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ShapeProblem {
    /// A shape with no area. Zero, negative, or one of the values a float can
    /// hold that is not a number at all.
    ///
    /// Both dimensions are reported together rather than one error each,
    /// because a shape is one thing and "it has no size" is one finding about
    /// it.
    #[error(
        "asset `{asset}`: a shape is {width}×{height} of the frame, and both have to be above zero"
    )]
    NotSized {
        /// The shape asset.
        asset: AssetId,
        /// Across, as written.
        width: f64,
        /// Down, as written.
        height: f64,
    },

    /// A corner rounder than the shape has room for.
    ///
    /// The radius is a fraction of the shape's shorter side, so `0.5` is the
    /// most there can be: at that point the two corners of that side have met
    /// and the end is a semicircle. Anything past it is asking for a rectangle
    /// with a negative straight edge, which is a number nobody meant.
    #[error("asset `{asset}`: corner radius {radius} is past the {max} a corner can be rounded")]
    RadiusOutOfRange {
        /// The shape asset.
        asset: AssetId,
        /// The fraction as written.
        radius: f64,
        /// The most a corner can be rounded.
        max: f64,
    },

    /// A border colour with no thickness to draw it at.
    ///
    /// Refused rather than ignored, because the two readings — *I meant no
    /// border* and *I meant a border and got the width wrong* — look identical
    /// in the rendered frame, and only one of them is what the document says.
    #[error("asset `{asset}`: a border is {width} thick, so nothing would be drawn")]
    BorderWithoutWidth {
        /// The shape asset.
        asset: AssetId,
        /// The thickness as written.
        width: f64,
    },

    /// A shape with neither an interior nor a border: a layer that would
    /// composite nothing over whatever is underneath.
    ///
    /// This is the one a render could not tell you about. An invisible layer
    /// and a layer that failed to draw look the same on screen, and a diagram
    /// missing one of its boxes is exactly the kind of thing nobody notices
    /// until the video is out.
    #[error("asset `{asset}`: a shape with no `fill` and no `stroke` would draw nothing")]
    Invisible {
        /// The shape asset.
        asset: AssetId,
    },
}
