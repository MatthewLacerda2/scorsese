//! Veo's published rates, as the vendor's own page lists them.
//!
//! Laid out to be read beside <https://ai.google.dev/gemini-api/docs/pricing>
//! and checked off row by row, which is the only way a hand-copied table stays
//! true. That is also why it holds tiers scorsese does not offer: the artifact
//! being audited is the vendor's price list, and a list missing rows is one
//! nobody can tick through.

use super::checked::Checked;

/// Which Veo tier a rate is for.
///
/// `Standard` is here because Google sells it, not because scorsese does —
/// [`VideoModel`](scorsese_core::VideoModel) offers `Fast` and `Lite` only.
/// Deleting the row would make this table cheaper and the audit impossible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// The full model. Not offered by scorsese.
    Standard,
    /// scorsese's default.
    Fast,
    /// Cheaper, and without reference images.
    Lite,
}

/// Which output size a rate is for.
///
/// `P4k` is the same case as [`Tier::Standard`]: published, not offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    /// 1280×720.
    P720,
    /// 1920×1080.
    P1080,
    /// 3840×2160.
    P4k,
}

/// What one combination costs, and when somebody last confirmed it.
///
/// The date is a field rather than a comment because a price with no date is a
/// price nobody can audit — and because a struct with two required fields
/// cannot be built with one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rate {
    /// US cents per second of finished video.
    ///
    /// Every rate Google publishes for Veo 3.1 is a whole number of cents per
    /// second, so this is exact and a total never rounds. If a vendor ever
    /// prices something at a third of a cent, this is the type that has to
    /// change, and it should change loudly.
    pub cents_per_second: u64,
    /// The day this figure was last read off the vendor's page.
    pub checked: Checked,
}

/// One line of the vendor's price list.
#[derive(Debug, Clone, Copy)]
pub struct Row {
    /// The tier the rate is for.
    pub tier: Tier,
    /// The output size the rate is for.
    pub quality: Quality,
    /// What it costs.
    pub rate: Rate,
}

/// The day the table below was last checked, whole.
const CHECKED: Checked = Checked::on(2026, 8, 4);

/// A row of the table.
const fn row(tier: Tier, quality: Quality, cents_per_second: u64) -> Row {
    Row {
        tier,
        quality,
        rate: Rate {
            cents_per_second,
            checked: CHECKED,
        },
    }
}

/// Veo 3.1, per second, paid tier. There is no free tier for video at all.
///
/// **Lite has no 4k row, and that is the point of a list rather than a grid.**
/// A grid would need something in that cell, and whatever went there would be
/// a price for something nobody can buy. Absent means absent, and
/// [`rate`] answers `None` for it.
pub const RATES: &[Row] = &[
    row(Tier::Standard, Quality::P720, 40),
    row(Tier::Standard, Quality::P1080, 40),
    row(Tier::Standard, Quality::P4k, 60),
    row(Tier::Fast, Quality::P720, 10),
    row(Tier::Fast, Quality::P1080, 12),
    row(Tier::Fast, Quality::P4k, 30),
    row(Tier::Lite, Quality::P720, 5),
    row(Tier::Lite, Quality::P1080, 8),
];

/// What this combination costs, or `None` if the vendor does not sell it.
pub fn rate(tier: Tier, quality: Quality) -> Option<Rate> {
    RATES
        .iter()
        .find(|row| row.tier == tier && row.quality == quality)
        .map(|row| row.rate)
}

impl Tier {
    /// How this tier is written in a table a person reads.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Standard => "Veo 3.1",
            Self::Fast => "Veo 3.1 Fast",
            Self::Lite => "Veo 3.1 Lite",
        }
    }

    /// Whether scorsese lets anybody choose this tier.
    ///
    /// Asked, never matched on at a call site — the same shape as
    /// [`VideoModel::supports_reference_images`](scorsese_core::VideoModel::supports_reference_images),
    /// so adding a tier is one arm here rather than a hunt through the crate.
    pub const fn is_offered(self) -> bool {
        !matches!(self, Self::Standard)
    }
}

impl Quality {
    /// How this size is written in a table a person reads.
    pub const fn label(self) -> &'static str {
        match self {
            Self::P720 => "720p",
            Self::P1080 => "1080p",
            Self::P4k => "4k",
        }
    }

    /// Whether scorsese lets anybody choose this size.
    pub const fn is_offered(self) -> bool {
        !matches!(self, Self::P4k)
    }
}

impl From<scorsese_core::VideoModel> for Tier {
    fn from(model: scorsese_core::VideoModel) -> Self {
        match model {
            scorsese_core::VideoModel::Fast => Self::Fast,
            scorsese_core::VideoModel::Lite => Self::Lite,
        }
    }
}

impl From<scorsese_core::VideoResolution> for Quality {
    fn from(resolution: scorsese_core::VideoResolution) -> Self {
        match resolution {
            scorsese_core::VideoResolution::P720 => Self::P720,
            scorsese_core::VideoResolution::P1080 => Self::P1080,
        }
    }
}
