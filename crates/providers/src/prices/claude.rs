//! Anthropic's published rates for the model the hosted assistant runs on.
//!
//! Laid out to be read beside <https://platform.claude.com/docs/en/about-claude/pricing>
//! and checked off row by row, the same way [`veo`](super::veo) and
//! [`elevenlabs`](super::elevenlabs) are. One model, because the assistant is
//! **Claude Opus 5.5** and is never downgraded to save credits (#527); a
//! second row is a second model somebody decided to run.
//!
//! # Exact, not estimated — the one table here that is
//!
//! Veo reports nothing and ElevenLabs is priced before the call. A Claude
//! response carries a `usage` block that counts every token it was billed for,
//! each kind at its own rate: plain input, output, cache writes (five-minute
//! and one-hour, at different prices) and cache reads. So [`Usage::micros`] is
//! the vendor's own count multiplied by the vendor's own page — as exact as a
//! copied rate table allows, and nothing is guessed.
//!
//! # Cents per million tokens
//!
//! The vendor prices per million tokens, and at that unit every figure is a
//! whole number of cents — $0.20 for a cache read is 20. Per token they would
//! be fractions of a micro-dollar. The division happens once, over the whole
//! call, in [`Usage::micros`], and it rounds **up** (`docs/prices.md`).

use super::checked::Checked;

/// The day every rate below was last read off the vendor's page.
///
/// All five columns were read in one sitting from one row of one table.
const CHECKED: Checked = Checked::on(2026, 9, 25);

/// The model the hosted assistant calls, as the API names it.
pub const MODEL: &str = "claude-opus-5-5";

/// What one model costs per million tokens of each kind, in US cents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rate {
    /// Input tokens that were neither written to nor read from the cache.
    pub input: u64,
    /// Output tokens, thinking included.
    pub output: u64,
    /// Input written to the five-minute cache: 1.25× input.
    pub cache_write_5m: u64,
    /// Input written to the one-hour cache: 2× input.
    pub cache_write_1h: u64,
    /// Input read from the cache: 0.05× input on this model, not the usual 0.1×.
    pub cache_read: u64,
    /// The day these figures were last read off the vendor's page.
    pub checked: Checked,
}

/// One line of the vendor's price list.
#[derive(Debug, Clone, Copy)]
pub struct Row {
    /// The model id the rate is for.
    pub model: &'static str,
    /// What it costs.
    pub rate: Rate,
}

/// Claude Opus 5.5 at the standard (not fast, not batch) tier, global routing.
///
/// `inference_geo: "us"` would cost 1.1× every column; the assistant does not
/// set it, so there is no row for it. Fast mode and the batch discount are the
/// same case: nothing here uses them.
pub const RATES: &[Row] = &[Row {
    model: MODEL,
    rate: Rate {
        input: 400,
        output: 2000,
        cache_write_5m: 500,
        cache_write_1h: 800,
        cache_read: 20,
        checked: CHECKED,
    },
}];

/// What this model costs, if the table has it.
///
/// `None` for a model with no row, so an assistant switched to a model nobody
/// priced is a refusal to answer for rather than a call charged at zero.
pub fn rate(model: &str) -> Option<Rate> {
    RATES
        .iter()
        .find(|row| row.model == model)
        .map(|row| row.rate)
}

/// The token counts one response reports, by the kind each is billed as.
///
/// The fields are the API's own `usage` fields: `input_tokens`,
/// `output_tokens`, `cache_read_input_tokens`, and `cache_creation`'s
/// `ephemeral_5m_input_tokens` and `ephemeral_1h_input_tokens`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Usage {
    /// Plain input tokens.
    pub input: u64,
    /// Output tokens.
    pub output: u64,
    /// Tokens written to the five-minute cache.
    pub cache_write_5m: u64,
    /// Tokens written to the one-hour cache.
    pub cache_write_1h: u64,
    /// Tokens read from the cache.
    pub cache_read: u64,
}

impl Usage {
    /// What these tokens cost at `rate`, in US micro-dollars, rounded up once.
    ///
    /// Summed in cent-tokens — tokens times cents per million — and divided
    /// once: a cent per million tokens is a hundredth of a micro-dollar per
    /// token, hence the hundred. Only the cache-read rate leaves a fraction (a
    /// fifth of a micro-dollar a token), and the call rounds it up. Saturates
    /// rather than wrapping on absurd counts.
    pub fn micros(&self, rate: Rate) -> u64 {
        let cent_tokens = [
            (self.input, rate.input),
            (self.output, rate.output),
            (self.cache_write_5m, rate.cache_write_5m),
            (self.cache_write_1h, rate.cache_write_1h),
            (self.cache_read, rate.cache_read),
        ]
        .iter()
        .fold(0u64, |sum, (tokens, cents)| {
            sum.saturating_add(tokens.saturating_mul(*cents))
        });
        cent_tokens.div_ceil(100)
    }
}
