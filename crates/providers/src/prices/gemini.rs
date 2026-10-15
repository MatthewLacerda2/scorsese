//! Nano Banana 2.1's published image rates, as the vendor's own page lists
//! them — the one image model scorsese uses (#1308).
//!
//! Laid out to be read beside <https://ai.google.dev/gemini-api/docs/pricing>
//! and checked off row by row, the way [`veo`](super::veo) is. Paid tier only:
//! no image model has a free tier.
//!
//! # Two units, because the vendor bills two things
//!
//! **The picture** is billed per image at a price fixed by its size — the page
//! gives it as $30 per million output tokens and then, helpfully, as a figure
//! per picture, which is what is copied here. **The input** — the prompt and
//! every reference picture — is billed per token, and is small: a reference is
//! about a twentieth of a cent. Neither is a whole number of cents, so the
//! table is kept in micro-dollars and the estimate rounds up once, at the end.

use scorsese_core::ImageResolution;

use super::checked::Checked;

/// The day every rate below was last read off the vendor's page.
const CHECKED: Checked = Checked::on(2026, 10, 15);

/// What one size costs, and when somebody last confirmed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rate {
    /// US micro-dollars per picture: $0.067 is `67_000`.
    pub microdollars_per_image: u64,
    /// US cents per million input tokens — text and pictures alike.
    pub cents_per_million_input: u64,
    /// The day this figure was last read off the vendor's page.
    pub checked: Checked,
}

/// One line of the vendor's price list.
#[derive(Debug, Clone, Copy)]
pub struct Row {
    /// The size the rate is for.
    pub resolution: ImageResolution,
    /// What it costs.
    pub rate: Rate,
}

/// A row, at the date at the top of this file.
const fn row(
    resolution: ImageResolution,
    microdollars_per_image: u64,
    cents_per_million_input: u64,
) -> Row {
    Row {
        resolution,
        rate: Rate {
            microdollars_per_image,
            cents_per_million_input,
            checked: CHECKED,
        },
    }
}

/// Every size Nano Banana 2.1 draws, standard (not batch) tier: $30 a million
/// output tokens, $1.50 a million input.
///
/// The pricing page (last updated 2026-10-14, read 2026-10-15) gives these per
/// picture. Its 1K and 2K figures are the image-generation page's token counts
/// at $30 a million — 1,120 and 1,680 tokens. Its 4K figure is not: 2,520
/// tokens would be $0.0756, and the page says $0.113. The billing page's
/// figure is the one kept, since an estimate that errs should err high.
pub const RATES: &[Row] = &[
    row(ImageResolution::K1, 33_600, 150),
    row(ImageResolution::K2, 50_400, 150),
    row(ImageResolution::K4, 113_000, 150),
];

/// What the **batch** tier costs, as a percentage of the standard row (#894).
///
/// Fifty: the page lists a batch column beside every standard one, picture and
/// input alike, and every batch figure is exactly half its standard
/// neighbour — so it is one number here rather than a second table that could
/// drift from the first. Read on the same day as the rows. The one exception
/// is 4K, listed at $0.0567 against half of $0.113: two hundredths of a cent,
/// which no estimate rounded to the cent can show, so it stays one number.
pub const BATCH_PERCENT: u64 = 50;

/// What this size costs, or `None` if the table has no row for it.
pub fn rate(resolution: ImageResolution) -> Option<Rate> {
    RATES
        .iter()
        .find(|row| row.resolution == resolution)
        .map(|row| row.rate)
}

/// How many input tokens one reference picture is counted as.
///
/// 1,120: what a Gemini 3 model charges for a picture at its default media
/// resolution (<https://ai.google.dev/gemini-api/docs/media-resolution>,
/// read 2026-10-02). scorsese never asks for another, so it is never another.
pub const TOKENS_PER_REFERENCE: u64 = 1_120;

/// How many characters of prompt one input token is counted as.
///
/// Four, the vendor's own rule of thumb for English. It is an approximation —
/// the prompt is never tokenised here — and at $1.50 a million it moves an
/// estimate by a few hundredths of a cent at most.
pub const CHARACTERS_PER_TOKEN: u64 = 4;
