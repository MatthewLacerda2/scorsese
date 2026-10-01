//! What a gradient can be asked for that could never be drawn.
//!
//! Its own catalogue because a fill is painted in two places — a shape's
//! interior and a `color` asset — and the finding is the same sentence in both,
//! with the field's name in it. Folding it into [`super::ShapeProblem`] would
//! leave the colour asset's gradient checked by nothing, or by a copy.
//!
//! Every one of these is answerable from the document alone, which is why the
//! offsets are fractions of the gradient rather than distances in pixels.

use crate::asset::AssetId;

/// One thing wrong with a gradient `fill` (or a `color` asset's gradient).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FillProblem {
    /// Fewer than two stops. One colour is a solid fill, and saying so as a
    /// string is the honest way to write it; none is nothing to paint.
    #[error(
        "asset `{asset}`: a gradient `{field}` has {count} stop(s) and needs at least two — \
         one colour is written as a plain \"#rrggbb\""
    )]
    TooFewStops {
        /// The asset carrying the gradient.
        asset: AssetId,
        /// Which field: `fill` on a shape, `color` on a colour asset.
        field: &'static str,
        /// How many stops it has.
        count: usize,
    },

    /// Stop offsets that go backwards or leave `0`–`1`.
    ///
    /// Two stops at the same place are allowed — that is a hard edge — but a
    /// stop placed before the one ahead of it has no reading every renderer
    /// agrees on, so it is refused rather than sorted into something nobody
    /// wrote.
    #[error(
        "asset `{asset}`: gradient `{field}` stops sit at {offsets:?}, and they have to rise \
         (or stay level) from 0 to 1"
    )]
    StopsOutOfOrder {
        /// The asset carrying the gradient.
        asset: AssetId,
        /// Which field.
        field: &'static str,
        /// Every stop's offset, as written.
        offsets: Vec<f64>,
    },

    /// A linear gradient's angle that is not a number.
    #[error("asset `{asset}`: gradient `{field}` has angle {angle}, which is not a direction")]
    BadAngle {
        /// The asset carrying the gradient.
        asset: AssetId,
        /// Which field.
        field: &'static str,
        /// The angle as written.
        angle: f64,
    },

    /// A radial gradient with no size, or a centre that is not a place.
    #[error(
        "asset `{asset}`: radial `{field}` is centred at ({x}, {y}) with radius {radius} — \
         the centre has to be numbers and the radius above zero"
    )]
    BadCircle {
        /// The asset carrying the gradient.
        asset: AssetId,
        /// Which field.
        field: &'static str,
        /// The centre's across, as written.
        x: f64,
        /// The centre's down, as written.
        y: f64,
        /// The radius as written.
        radius: f64,
    },
}
