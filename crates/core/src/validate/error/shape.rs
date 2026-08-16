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
use crate::shape::Point;
use crate::timeline::ClipId;

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

    /// An arrow endpoint that is not a pair of numbers — a coordinate no frame
    /// could be measured against at any resolution.
    ///
    /// A point *outside* the frame is not this and is not refused: an arrow
    /// entering from off-screen is an ordinary thing to draw.
    #[error(
        "asset `{asset}`: an arrow runs from ({}, {}) to ({}, {}), which is not a pair of places",
        from.x, from.y, to.x, to.y
    )]
    Unplaced {
        /// The arrow asset.
        asset: AssetId,
        /// The start, as written.
        from: Point,
        /// The end, as written.
        to: Point,
    },

    /// An arrow whose two ends are in the same place.
    ///
    /// No length to draw, and no direction to aim a head along — so what
    /// reaches the screen is either nothing at all or a smudge of ink where the
    /// head landed. Both readings are somebody having meant a second point.
    #[error(
        "asset `{asset}`: an arrow starts and ends at ({}, {}), so it has no direction",
        at.x, at.y
    )]
    NoLength {
        /// The arrow asset.
        asset: AssetId,
        /// The place both ends sit at.
        at: Point,
    },

    /// A `fill` on an arrow.
    ///
    /// A line encloses nothing, so there is no inside for a colour to go in.
    /// Refused rather than ignored for the reason every stray field is: nothing
    /// would read it, and silence about it would look exactly like having drawn
    /// it. Usually a `stroke` that was meant.
    #[error("asset `{asset}`: an arrow is a line and has no inside, so `fill` would draw nothing")]
    FilledLine {
        /// The arrow asset.
        asset: AssetId,
    },

    /// An arrow attached to a clip the timeline does not have.
    ///
    /// Usually a clip renamed or deleted with the arrow left behind — which is
    /// exactly the edit this whole feature exists to survive, so it is worth
    /// saying loudly rather than quietly drawing the arrow at the origin.
    #[error("asset `{asset}`: an arrow is attached to clip `{clip}`, which is not in the timeline")]
    AttachedToNothing {
        /// The arrow asset.
        asset: AssetId,
        /// The clip id it names.
        clip: ClipId,
    },

    /// An arrow attached to a clip on an audio track.
    ///
    /// Sound has no rectangle. Nothing about where a piece of music sits could
    /// answer where an arrow should point.
    #[error(
        "asset `{asset}`: an arrow is attached to clip `{clip}`, which is sound and has no place on screen"
    )]
    AttachedToSound {
        /// The arrow asset.
        asset: AssetId,
        /// The clip id it names.
        clip: ClipId,
    },

    /// An arrow attached to a clip that is itself an arrow.
    ///
    /// **This is the rule that forecloses every cycle**, which is why it is a
    /// blanket refusal rather than a graph walk: an arrow following an arrow
    /// following the first has no answer, and nothing resolving endpoints per
    /// frame should have to discover that at render time. It costs nothing real
    /// — a line has no rectangle worth meeting — and it covers the degenerate
    /// case of an arrow attached to a clip showing itself.
    #[error(
        "asset `{asset}`: an arrow is attached to clip `{clip}`, which is another arrow — a line has no side to meet"
    )]
    AttachedToArrow {
        /// The arrow asset.
        asset: AssetId,
        /// The clip id it names.
        clip: ClipId,
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
