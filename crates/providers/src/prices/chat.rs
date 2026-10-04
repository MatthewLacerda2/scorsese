//! What the hosted assistant's models cost, per million tokens (#540, #705).
//!
//! Laid out to be read beside the vendors' own pages and checked off row by
//! row, the way [`veo`](super::veo) and [`elevenlabs`](super::elevenlabs) are:
//! <https://platform.claude.com/docs/en/about-claude/pricing> for Claude, and
//! <https://ai.google.dev/gemini-api/docs/pricing> for Gemini. One row per
//! model the assistant offers ([`Model`]); a model with no row cannot be
//! charged for, and so cannot be offered.
//!
//! # Exact, not estimated — the one table here that is
//!
//! Veo reports nothing and ElevenLabs is priced before the call. A chat reply
//! carries a usage block that counts every token it was billed for, and
//! [`Usage`] holds those counts by the kind each is billed as: plain input,
//! output (thinking included, on both vendors), cache writes (Claude's
//! five-minute and one-hour, at different prices) and cache reads. So
//! [`Usage::micros`] is the vendor's own count multiplied by the vendor's own
//! page — as exact as a copied rate table allows, and nothing is guessed.
//!
//! **A cache read is always charged at the cache-read rate**, never as full
//! input: the saving is the user's, not the operator's (the maintainer,
//! 2026-10-03, on #705).
//!
//! # Micro-dollars per million tokens
//!
//! Claude's figures are whole cents per million, but Gemini 3.8 Flash's
//! cached input is $0.075 — seven and a half cents — so the unit is the
//! micro-dollar per million tokens, where every published figure is a whole
//! number. The division happens once, over the whole call, in
//! [`Usage::micros`], and it rounds **up** (`docs/prices.md`).

use super::checked::Checked;
use crate::chat::Model;

/// What one model costs per million tokens of each kind, in US micro-dollars:
/// $4 is `4_000_000`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rate {
    /// Input tokens that were neither written to nor read from a cache.
    pub input: u64,
    /// Output tokens, thinking included.
    pub output: u64,
    /// Input written to Claude's five-minute cache: 1.25× input. Zero on
    /// Gemini, whose implicit cache is written for nothing.
    pub cache_write_5m: u64,
    /// Input written to Claude's one-hour cache: 2× input. Zero on Gemini.
    pub cache_write_1h: u64,
    /// Input read from a cache.
    pub cache_read: u64,
    /// The day these figures were last read off the vendor's page.
    pub checked: Checked,
}

/// One line of the price list.
#[derive(Debug, Clone, Copy)]
pub struct Row {
    /// The model the rate is for.
    pub model: Model,
    /// What it costs.
    pub rate: Rate,
}

/// `dollars` and `millionths` of a dollar, in micro-dollars.
const fn usd(dollars: u64, millionths: u64) -> u64 {
    dollars * 1_000_000 + millionths
}

/// Every model the assistant offers, at the standard tier (not batch, not
/// fast, not priority), global routing.
///
/// - **Claude Opus 5.5**: cache reads are 0.05× input on this model, not the
///   usual 0.1× (checked 2026-10-03 against Anthropic's prompt-caching page).
/// - **Claude Sonnet 5.5**: the usual multipliers — 1.25×, 2× and **0.1×**.
/// - **Gemini 3.8 Flash**: an introductory price **that doubles on
///   2027-01-01** (to $1.50 / $7.50 / $0.15); the row must be re-read then.
///   Implicit caching is automatic on this model, writes cost nothing, and
///   reads are 0.1× input. (Explicit caches, with their hourly storage fee,
///   are never created.)
/// - **Gemini 3.5 Flash Lite**: Google sells no context caching on it, and
///   the model is not on the implicit-caching list. A cached count, should
///   one ever appear, is billed as the plain input it would then be.
pub const RATES: &[Row] = &[
    Row {
        model: Model::ClaudeOpus55,
        rate: Rate {
            input: usd(4, 0),
            output: usd(20, 0),
            cache_write_5m: usd(5, 0),
            cache_write_1h: usd(8, 0),
            cache_read: usd(0, 200_000),
            checked: Checked::on(2026, 10, 3),
        },
    },
    Row {
        model: Model::ClaudeSonnet55,
        rate: Rate {
            input: usd(2, 0),
            output: usd(10, 0),
            cache_write_5m: usd(2, 500_000),
            cache_write_1h: usd(4, 0),
            cache_read: usd(0, 200_000),
            checked: Checked::on(2026, 10, 3),
        },
    },
    Row {
        model: Model::GeminiFlash38,
        rate: Rate {
            input: usd(0, 750_000),
            output: usd(3, 750_000),
            cache_write_5m: 0,
            cache_write_1h: 0,
            cache_read: usd(0, 75_000),
            checked: Checked::on(2026, 10, 3),
        },
    },
    Row {
        model: Model::GeminiFlashLite35,
        rate: Rate {
            input: usd(0, 300_000),
            output: usd(2, 500_000),
            cache_write_5m: 0,
            cache_write_1h: 0,
            cache_read: usd(0, 300_000),
            checked: Checked::on(2026, 10, 3),
        },
    },
];

/// What `model` costs, if the table has it.
///
/// `None` for a model with no row, so a call nobody priced is a refusal to
/// answer for rather than a call charged at zero.
pub fn rate(model: Model) -> Option<Rate> {
    RATES
        .iter()
        .find(|row| row.model == model)
        .map(|row| row.rate)
}

/// The token counts one reply reports, by the kind each is billed as — the
/// same five on every vendor, so a call is recorded the same way whoever
/// answered it.
///
/// From Claude's `usage`: `input_tokens`, `output_tokens`,
/// `cache_read_input_tokens`, and `cache_creation`'s
/// `ephemeral_5m_input_tokens` and `ephemeral_1h_input_tokens`. From Gemini's
/// `usageMetadata`: input is `promptTokenCount` less
/// `cachedContentTokenCount`, output is `candidatesTokenCount` plus
/// `thoughtsTokenCount`, and the cached count is the cache read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Usage {
    /// Plain input tokens.
    pub input: u64,
    /// Output tokens, thinking included.
    pub output: u64,
    /// Tokens written to the five-minute cache.
    pub cache_write_5m: u64,
    /// Tokens written to the one-hour cache.
    pub cache_write_1h: u64,
    /// Tokens read from a cache.
    pub cache_read: u64,
}

impl Usage {
    /// What these tokens cost at `rate`, in US micro-dollars, rounded up once.
    ///
    /// Summed in micro-dollar-tokens — tokens times micro-dollars per million
    /// — and divided by the million once, so a fraction of a micro-dollar is
    /// rounded up over the whole call rather than per kind. Saturates rather
    /// than wrapping on absurd counts.
    pub fn micros(&self, rate: Rate) -> u64 {
        [
            (self.input, rate.input),
            (self.output, rate.output),
            (self.cache_write_5m, rate.cache_write_5m),
            (self.cache_write_1h, rate.cache_write_1h),
            (self.cache_read, rate.cache_read),
        ]
        .iter()
        .fold(0u64, |sum, (tokens, micros)| {
            sum.saturating_add(tokens.saturating_mul(*micros))
        })
        .div_ceil(1_000_000)
    }
}
