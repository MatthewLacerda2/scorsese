//! The recharge cards the public pricing page shows (#1538): the least a
//! purchase may be and each bonus tier's first amount, each quoted at the
//! rate kept now, so a card says the dollars it buys — and a card's Buy names
//! that rate, so what the card said is what the purchase locks.

use serde::Serialize;

use super::begin::quoted;
use super::{MAXIMUM_CENTAVOS, MINIMUM_CENTAVOS, Payments, Quote};

/// One card: an amount of reais, and what it buys now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Card {
    /// The reais, in centavos.
    pub brl_centavos: i64,
    /// The bonus it adds, as a percentage of its credits.
    pub bonus_percent: i64,
    /// Whether it is the card with the most bonus, the one the page
    /// highlights.
    pub best: bool,
    /// What it buys at the rate kept now, bonus included — `None` while no
    /// rate fresh enough to sell at is kept.
    pub quote: Option<Quote>,
}

/// What buying credits looks like to a visitor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Recharge {
    /// The least one purchase may be, in centavos.
    pub minimum_centavos: i64,
    /// The most one purchase may be, in centavos.
    pub maximum_centavos: i64,
    /// The bonus tiers, lowest first.
    pub tiers: Vec<super::bonus::Tier>,
    /// The cards, smallest first.
    pub cards: Vec<Card>,
}

impl Payments {
    /// The cards at the rate kept now. Reads no rate: a visitor's page never
    /// waits on a rate source, and a server that has none fresh shows the
    /// cards without their dollars until [`refresh`](super::refresh) reads one.
    pub fn recharge(&self) -> Recharge {
        let rate = self.rates.current();
        let tiers = self.bonus.tiers().to_vec();
        let mut amounts: Vec<i64> = std::iter::once(MINIMUM_CENTAVOS)
            .chain(tiers.iter().map(|tier| tier.from_centavos))
            .filter(|centavos| (MINIMUM_CENTAVOS..=MAXIMUM_CENTAVOS).contains(centavos))
            .collect();
        amounts.sort_unstable();
        amounts.dedup();
        let most = amounts
            .iter()
            .map(|centavos| self.bonus.percent(*centavos))
            .max()
            .unwrap_or(0);
        let cards = amounts
            .into_iter()
            .map(|centavos| {
                let percent = self.bonus.percent(centavos);
                Card {
                    brl_centavos: centavos,
                    bonus_percent: percent,
                    best: percent > 0 && percent == most,
                    quote: rate
                        .as_ref()
                        .and_then(|rate| quoted(centavos, rate, &self.bonus).ok()),
                }
            })
            .collect();
        Recharge {
            minimum_centavos: MINIMUM_CENTAVOS,
            maximum_centavos: MAXIMUM_CENTAVOS,
            tiers,
            cards,
        }
    }
}
