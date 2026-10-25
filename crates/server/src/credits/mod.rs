//! Credits: what each user has paid in and spent, and the record of every
//! paid generation (#537).
//!
//! Users pay up front; the maintainer pays Google, ElevenLabs and Anthropic up
//! front; every cent in between is accounted for **here, in the database** —
//! not in a provider's billing export a day late. The doctrine is
//! `docs/web.md` (*Money*); this is how it is kept.
//!
//! ## The ledger is append-only, and the database holds it so
//!
//! `credit_entries` is the truth, and a user's balance is the **sum** of their
//! entries. An entry is never edited; a correction is a new entry. That is
//! enforced by Postgres rather than by care: the member role has no `UPDATE`
//! or `DELETE` on the table at all, and a trigger refuses both to everybody
//! else too — the login role included — except the delete that cascades from
//! an account's own deletion (migration `0004_credits.sql`).
//!
//! Amounts are **integer micro-dollars** in a signed `BIGINT`: money in is
//! positive, money out negative. One assistant call can cost a fraction of a
//! cent, so cents are too coarse; a float is not the same sum twice.
//!
//! ## Spending: reserve, then settle
//!
//! A paid generation ([`generations`]) is priced by the provider tables in
//! `scorsese_providers::prices` — the same figure the quote (#538) showed —
//! times its [`Product`]'s multiplier ([`price`]). Then:
//!
//! 1. **Reserve** the price: a negative entry, written only when the balance
//!    covers it. So two generations started at once cannot both spend the
//!    last dollar — each takes a lock on the user's own `users` row first.
//!    Refused means nothing was sent to the provider and nothing is recorded.
//! 2. **Settle** once the provider answers. A release gives the reservation
//!    back; if the generation **worked**, a charge takes the price. So a
//!    **failure is free** and a success is charged, liked or not (#527).
//!    A unique index allows one release per reservation, so nothing is
//!    settled twice — the ledger's own form of "never pay twice".
//!
//! A Veo job that goes `stuck` settles nothing: Google may still be
//! generating and billing, so the reservation stays held until the shot is
//! collected or given up on.
//!
//! **The assistant is charged after its call, not reserved for**
//! ([`ledger::charge_assistant`]): its cost is exact only once the response
//! has counted its tokens. Whoever runs a turn (#540) checks there is a
//! positive balance before starting one; the call itself may take the balance
//! a little below zero, which the next top-up absorbs.
//!
//! ## Per generation, and nothing else
//!
//! There is **no monthly fee** (#1243, the maintainer, 2026-10-13): a user pays
//! for what they generate and for nothing while they do not. What scorsese
//! spends on a user is third-party APIs, so that is what it charges in line
//! with; electricity, time, the machine and its disk are deliberately left
//! unpriced.
//!
//! ## What the user sees
//!
//! [`history`] is the same ledger scoped to its owner and written for them:
//! one row per thing that moved their balance, the balance after each, in
//! dollars — the currency the ledger is kept in, and the only one shown
//! (#703): a converted figure is wrong the day after it is shown. It is served
//! at `GET /api/credits/history` and described as a read-only tool
//! ([`tool`]), which web MCP (#539, `crate::tools`) serves.

pub mod allowance;
pub mod command;
pub mod designs;
pub mod generations;
pub mod history;
pub mod ledger;
pub mod lines;
pub mod tool;
pub mod tracks;

pub use ledger::Reservation;

use scorsese_core::{Asset, AssetKind, AudioBlock};
pub use scorsese_providers::prices::{MICROS_PER_CENT, from_cents};

/// What a user buys, each kind priced by its own multiplier (#1247).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Product {
    /// A Veo shot.
    VeoShot,
    /// A Gemini still, drawn now or in a half-price batch.
    Still,
    /// An ElevenLabs spoken line.
    SpokenLine,
    /// An ElevenLabs Music track (#1264).
    MusicTrack,
    /// An ElevenLabs voice design.
    VoiceDesign,
    /// One call to the assistant's model.
    Assistant,
}

impl Product {
    /// What a generated asset is sold as, or `None` for a kind the user is
    /// never charged for: everything that is not prompted
    /// (`AssetKind::is_prompted`). A voice design makes no asset, so no asset
    /// is one. A `generated_audio` asset is a line or a track by the block it
    /// carries (`Asset::audio_block`).
    pub fn of(asset: &Asset) -> Option<Self> {
        match asset.kind {
            AssetKind::GeneratedVideo => Some(Self::VeoShot),
            AssetKind::GeneratedImage => Some(Self::Still),
            AssetKind::GeneratedAudio => match asset.audio_block() {
                Some(AudioBlock::Music) => Some(Self::MusicTrack),
                _ => Some(Self::SpokenLine),
            },
            AssetKind::Video
            | AssetKind::Image
            | AssetKind::Audio
            | AssetKind::Text
            | AssetKind::Color
            | AssetKind::Shape
            | AssetKind::Icon
            | AssetKind::ImageSequence
            | AssetKind::Html
            | AssetKind::SynthAudio
            | AssetKind::Group => None,
        }
    }

    /// What the user is charged for it, as a percentage of what it costs
    /// scorsese: **the one table of prices** (the maintainer, 2026-10-13).
    ///
    /// Every dollar of API spend passes three costs that scale with it: the
    /// Brazilian card that pays the vendor (IOF and the card's spread), the
    /// tax on the revenue (Simples Nacional, 6% or 15.5% — not known yet) and
    /// the fee for taking the user's payment. ×1.1 lost money in every case;
    /// these leave a margin in every case, thinner on video, the most
    /// expensive thing a user buys. `docs/web.md` (*Credits*, *Pricing*) has
    /// the derivation. **Expected to change**: re-derive when the tax rate,
    /// the card spread or a vendor's price changes.
    pub const fn percent(self) -> u64 {
        match self {
            Self::VeoShot => 150,
            Self::Still
            | Self::SpokenLine
            | Self::MusicTrack
            | Self::VoiceDesign
            | Self::Assistant => 180,
        }
    }
}

/// What a user is charged for a `product` that costs scorsese `cost_micros`:
/// the cost times [`Product::percent`], **rounded up** (`docs/prices.md`) —
/// rounding down would give away a fraction of a micro-dollar at a time.
pub fn price(product: Product, cost_micros: u64) -> i64 {
    let marked = u128::from(cost_micros) * u128::from(product.percent());
    i64::try_from(marked.div_ceil(100)).unwrap_or(i64::MAX)
}

/// What a user is charged for a `product` whose provider table says it costs
/// scorsese `cents`: [`price`] over [`from_cents`], the one way a quote, an
/// offer and a reservation all reach the same figure.
pub fn charge_for(product: Product, cents: u64) -> i64 {
    price(product, from_cents(cents))
}

/// Micro-dollars as dollars a person reads: `$1.06`, `-$0.03`, `$1,234.00`,
/// and as many places past the cent as a fraction of one needs — `$0.0022`.
/// [`prices::dollars`](scorsese_providers::prices::dollars) at two places,
/// the one formatter every surface prints money with (#1287).
pub fn dollars(micros: impl Into<i128>) -> String {
    scorsese_providers::prices::dollars(micros, 2)
}

/// Why spending or recording credits did not happen.
#[derive(Debug, thiserror::Error)]
pub enum CreditError {
    /// The balance does not cover what this would reserve. Nothing was spent.
    #[error(
        "your balance is {}, and this needs {} — nothing was sent to the provider. \
         Add credit to go ahead.",
        dollars(*balance),
        dollars(*needed)
    )]
    Insufficient {
        /// What the balance was.
        balance: i64,
        /// What the generation would have reserved.
        needed: i64,
    },
    /// A reservation that does not exist, or was already settled.
    #[error("reservation {0} is not open: it does not exist or was already settled")]
    NotOpen(i64),
    /// The assistant ran on a model the rate table has no row for.
    #[error("there is no published rate for {0}, so the call cannot be charged")]
    Unpriced(String),
    /// An operator's figure that does not parse, or makes no sense.
    #[error("{0}")]
    Invalid(String),
    /// The database refused.
    #[error("the database refused: {0}")]
    Database(#[from] sqlx::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    mod documented;

    fn of(kind: AssetKind) -> Option<Product> {
        Product::of(&Asset::sketch(scorsese_core::AssetId::new("a"), kind, "p"))
    }

    /// Only a prompted kind is sold, each as the product its reservation is
    /// made as (`generations::start`).
    #[test]
    fn a_prompted_kind_is_sold_and_nothing_else_is() {
        assert_eq!(of(AssetKind::GeneratedVideo), Some(Product::VeoShot));
        assert_eq!(of(AssetKind::GeneratedImage), Some(Product::Still));
        assert_eq!(of(AssetKind::GeneratedAudio), Some(Product::SpokenLine));
        assert_eq!(of(AssetKind::SynthAudio), None);
        assert_eq!(of(AssetKind::Image), None);
    }

    /// A `generated_audio` asset with a `music` block is a track (#1264).
    #[test]
    fn a_track_is_sold_as_one() {
        let mut track = Asset::sketch(
            scorsese_core::AssetId::new("bed"),
            AssetKind::GeneratedAudio,
            "lo-fi",
        );
        track.music = Some(scorsese_core::MusicRequest::default());
        assert_eq!(Product::of(&track), Some(Product::MusicTrack));
    }

    /// A charge is the cents in micro-dollars, marked up and rounded up.
    #[test]
    fn a_charge_is_the_price_of_the_cents() {
        assert_eq!(charge_for(Product::VeoShot, 40), 600_000);
        assert_eq!(charge_for(Product::Still, 4), 72_000);
        assert_eq!(
            charge_for(Product::Still, 4),
            price(Product::Still, from_cents(4))
        );
    }
}
