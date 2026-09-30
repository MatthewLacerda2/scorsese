//! What a text asset's reveal and counter blocks can say that could never be
//! drawn as meant.
//!
//! Its own catalogue for [`super::ShapeProblem`]'s reason: these are findings
//! about the numbers *inside* one block, and the assets catalogue is where
//! whether a block is there at all is reported.

use crate::asset::AssetId;

/// One thing wrong with how a text asset reveals or counts.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TextProblem {
    /// A stagger outside `0..=1`. Below zero would start a piece before the one
    /// ahead of it, and above one would leave a gap in which nothing arrives —
    /// neither is an overlap, which is what the number is.
    #[error("asset `{asset}`: a reveal's stagger is {stagger}, and it has to be from 0 to 1")]
    StaggerOutOfRange {
        /// The text asset.
        asset: AssetId,
        /// The number as written.
        stagger: f64,
    },

    /// A rise that is not a number. Any finite one is a distance — negative
    /// drops the pieces in from above — but an infinity is not.
    #[error("asset `{asset}`: a reveal's rise is {rise}, which is not a distance")]
    RiseNotFinite {
        /// The text asset.
        asset: AssetId,
        /// The number as written.
        rise: f64,
    },

    /// A `number` block on a text with nowhere to put the figure.
    ///
    /// Refused rather than ignored: a counter that shows nothing looks exactly
    /// like a counter that is broken, and the missing `{n}` is the one thing
    /// the author has to add.
    #[error("asset `{asset}`: it has a `number` but its text has no `{{n}}` to write it at")]
    NoPlaceholder {
        /// The text asset.
        asset: AssetId,
    },

    /// A figure that is not a number.
    #[error("asset `{asset}`: its `number` value is {value}, which cannot be written as a figure")]
    ValueNotFinite {
        /// The text asset.
        asset: AssetId,
        /// The value as written.
        value: f64,
    },

    /// More decimals than a figure is written with.
    #[error("asset `{asset}`: {decimals} decimals is more than the {max} a figure can carry")]
    TooManyDecimals {
        /// The text asset.
        asset: AssetId,
        /// The count as written.
        decimals: u8,
        /// The most there may be.
        max: u8,
    },
}
