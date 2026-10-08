//! Gemini's published image rates, as the vendor's own page lists them.
//!
//! Laid out to be read beside <https://ai.google.dev/gemini-api/docs/pricing>
//! and checked off row by row, the way [`veo`](super::veo) is. Paid tier only:
//! no image model has a free tier.
//!
//! # Two units, because the vendor bills two things
//!
//! **The picture** is billed per image at a price fixed by its size — the page
//! gives it as $60 per million output tokens and then, helpfully, as a figure
//! per picture, which is what is copied here. **The input** — the prompt and
//! every reference picture — is billed per token, and is small: a reference is
//! about a twentieth of a cent. Neither is a whole number of cents, so the
//! table is kept in micro-dollars and the estimate rounds up once, at the end.

use scorsese_core::{ImageModel, ImageResolution};

use super::checked::Checked;

/// The day every rate below was last read off the vendor's page.
const CHECKED: Checked = Checked::on(2026, 10, 8);

/// What one size from one model costs, and when somebody last confirmed it.
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
    /// The model the rate is for.
    pub model: ImageModel,
    /// The size the rate is for.
    pub resolution: ImageResolution,
    /// What it costs.
    pub rate: Rate,
}

/// A row, at the date at the top of this file.
const fn row(
    model: ImageModel,
    resolution: ImageResolution,
    microdollars_per_image: u64,
    cents_per_million_input: u64,
) -> Row {
    Row {
        model,
        resolution,
        rate: Rate {
            microdollars_per_image,
            cents_per_million_input,
            checked: CHECKED,
        },
    }
}

/// Every image model scorsese offers, standard (not batch) tier.
///
/// A model has a row for each size it draws; absent means absent, and [`rate`]
/// answers `None` for the rest — so **Lite has one row** and only Flash has a
/// 0.5K one. Nano Banana 2.1's output is $30 a million against Flash's $60,
/// which is why it is cheaper per picture at every size it draws although its
/// input costs three times as much ($1.50 a million against $0.50). Pro's page
/// gives one price for 1K and 2K, so its two rows are the same figure.
pub const RATES: &[Row] = &[
    row(ImageModel::NanoBanana21, ImageResolution::K1, 33_600, 150),
    row(ImageModel::NanoBanana21, ImageResolution::K2, 50_400, 150),
    row(ImageModel::NanoBanana21, ImageResolution::K4, 113_000, 150),
    row(ImageModel::Flash, ImageResolution::K05, 45_000, 50),
    row(ImageModel::Flash, ImageResolution::K1, 67_000, 50),
    row(ImageModel::Flash, ImageResolution::K2, 101_000, 50),
    row(ImageModel::Flash, ImageResolution::K4, 151_000, 50),
    row(ImageModel::Lite, ImageResolution::K1, 33_600, 25),
    row(ImageModel::Pro, ImageResolution::K1, 134_000, 200),
    row(ImageModel::Pro, ImageResolution::K2, 134_000, 200),
    row(ImageModel::Pro, ImageResolution::K4, 240_000, 200),
];

/// What the **batch** tier costs, as a percentage of the standard row (#894).
///
/// Fifty: the page lists a batch column beside every standard one, picture and
/// input alike, and every batch figure is exactly half its standard
/// neighbour — so it is one number here rather than a second table that could
/// drift from the first. Read on the same day as the rows; the day a batch
/// figure is not half its neighbour, this becomes a table.
pub const BATCH_PERCENT: u64 = 50;

/// What this model costs at this size, or `None` if the vendor does not sell it.
pub fn rate(model: ImageModel, resolution: ImageResolution) -> Option<Rate> {
    RATES
        .iter()
        .find(|row| row.model == model && row.resolution == resolution)
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
/// the prompt is never tokenised here — and even at Pro's $2 a million it moves
/// an estimate by a few hundredths of a cent at most.
pub const CHARACTERS_PER_TOKEN: u64 = 4;
