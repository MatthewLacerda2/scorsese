//! ElevenLabs' published rates, as the vendor's own page lists them.
//!
//! Laid out to be read beside <https://elevenlabs.io/pricing> and checked off
//! row by row, the same way [`veo`](super::veo) is. Shorter, because the vendor
//! prices text-to-speech by one thing only: how many characters go in.
//!
//! # Priced before the call, not after it
//!
//! This is the one real difference from video, and it is a pleasant one. A shot
//! is priced by its length, which the request fixes; a line is priced by its
//! text, which is *already in the document*. So the figure is exact and known
//! before anything is sent — no estimate that only settles once the vendor has
//! answered.
//!
//! It is still called an estimate, and for two reasons that survive the
//! arithmetic being exact. The rate is a page somebody copied, and a handful of
//! Voice Library voices carry a credit multiplier that scorsese deliberately
//! ignores — see [`super`] for that decision. So the number can be a little
//! low, and it is never a bill.

use super::checked::Checked;

/// The day every rate below was last read off the vendor's page.
///
/// One constant rather than a date per row, because they were all read in one
/// sitting from one page. The moment they stop being, this becomes a field.
const CHECKED: Checked = Checked::on(2026, 8, 5);

/// Which text-to-speech model a rate is for.
///
/// The Turbo variants are absent, and that is a departure from how
/// [`veo`](super::veo) handles a tier it does not sell. Veo's table keeps
/// `Standard` so the vendor's page can be ticked through. Turbo is different:
/// the vendor's own documentation recommends Flash over Turbo in every case, so
/// it is not a row scorsese declines to offer — it is a row the vendor tells
/// people not to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    /// `eleven_v3` — the most expressive.
    Expressive,
    /// `eleven_multilingual_v2` — the vendor's default.
    Standard,
    /// `eleven_flash_v2_5` — low latency, half price.
    Fast,
}

impl Model {
    /// The model as somebody reading a price table expects it.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Expressive => "expressive",
            Self::Standard => "standard",
            Self::Fast => "fast",
        }
    }

    /// The vendor's id for this model, which is what a price table names it by.
    ///
    /// Delegates rather than repeating the strings. There is exactly one place
    /// scorsese's word for a model becomes the vendor's, and it is not going to
    /// be two — the first person to notice they disagreed would be the one who
    /// had paid for a reading in the wrong model.
    pub fn model_id(self) -> &'static str {
        scorsese_core::SpeechModel::from(self).model_id()
    }
}

impl From<Model> for scorsese_core::SpeechModel {
    fn from(model: Model) -> Self {
        match model {
            Model::Expressive => Self::Expressive,
            Model::Standard => Self::Standard,
            Model::Fast => Self::Fast,
        }
    }
}

impl From<scorsese_core::SpeechModel> for Model {
    fn from(model: scorsese_core::SpeechModel) -> Self {
        match model {
            scorsese_core::SpeechModel::Expressive => Self::Expressive,
            scorsese_core::SpeechModel::Standard => Self::Standard,
            scorsese_core::SpeechModel::Fast => Self::Fast,
        }
    }
}

/// What one model costs, and when somebody last confirmed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rate {
    /// US cents per thousand characters of input text.
    ///
    /// Per **thousand** rather than per character because the vendor's figures
    /// are whole cents at that unit — 10¢ and 5¢ — and per character they would
    /// be thousandths. The division happens once, in [`super::speech`], where it
    /// rounds up.
    pub cents_per_1k_chars: u64,
    /// The day this figure was last read off the vendor's page.
    pub checked: Checked,
}

/// One line of the vendor's price list.
#[derive(Debug, Clone, Copy)]
pub struct Row {
    /// The model the rate is for.
    pub model: Model,
    /// What it costs.
    pub rate: Rate,
}

/// One row, at the date at the top of this file.
const fn row(model: Model, cents_per_1k_chars: u64) -> Row {
    Row {
        model,
        rate: Rate {
            cents_per_1k_chars,
            checked: CHECKED,
        },
    }
}

/// Every rate the vendor publishes for the models scorsese speaks with.
pub const RATES: &[Row] = &[
    row(Model::Expressive, 10),
    row(Model::Standard, 10),
    row(Model::Fast, 5),
];

/// What this model costs, if it is sold.
///
/// An `Option` for symmetry with [`veo::rate`](super::veo::rate), where a
/// combination genuinely can be unsold. Here every model has a rate, and the
/// shape is kept so that a model added without a price is a `None` somebody has
/// to answer for rather than a zero nobody notices.
pub fn rate(model: Model) -> Option<Rate> {
    RATES
        .iter()
        .find(|row| row.model == model)
        .map(|row| row.rate)
}
