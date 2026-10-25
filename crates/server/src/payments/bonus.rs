//! Bonus credit on larger recharges (#1537): the maintainer's way of taking
//! a thinner margin from people who put in more.
//!
//! **A bonus, not a lower multiplier.** Credits are one pool, so a price that
//! depended on the purchase would need to know which purchase each spent
//! micro-dollar came from; a bonus is the same economics and one more entry.
//! It is a share of the credits the reais buy, floored, by the highest tier
//! the amount reaches, and it is **locked on the payment** with the rate, so
//! what the person was shown is what they get even if the tiers change while
//! they pay. Settling credits it as its own `bonus` entry beside the top-up,
//! once per payment by a unique index.
//!
//! **It stops at 10%**: at ×1.5 a Veo shot with a 10% bonus is about
//! break-even under the higher tax bracket (docs/web.md, *Credits*).
//!
//! The tiers are configuration ([`RECHARGE_BONUS`]), never constants here.

use scorsese_providers::credentials::Environment;
use serde::Serialize;

use crate::config::{ConfigError, RECHARGE_BONUS};

/// One tier: from so many reais, so much more credit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Tier {
    /// The least a purchase reaching it is, in centavos.
    pub from_centavos: i64,
    /// The credit added, as a percentage of what the reais buy.
    pub percent: i64,
}

/// The tiers, lowest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Bonus(Vec<Tier>);

impl Default for Bonus {
    /// The maintainer's tiers (2026-10-24): +5% from R$ 40, +10% from R$ 80.
    fn default() -> Self {
        Self(vec![
            Tier {
                from_centavos: 4_000,
                percent: 5,
            },
            Tier {
                from_centavos: 8_000,
                percent: 10,
            },
        ])
    }
}

impl Bonus {
    /// No bonus on anything.
    pub fn none() -> Self {
        Self(Vec::new())
    }

    /// What [`RECHARGE_BONUS`] says, [`Bonus::default`] when unset.
    pub fn from_environment(environment: &Environment) -> Result<Self, ConfigError> {
        match environment.get(RECHARGE_BONUS) {
            None => Ok(Self::default()),
            Some(value) => Self::parse(value).ok_or_else(|| ConfigError::Bonus {
                value: value.to_owned(),
            }),
        }
    }

    /// Tiers written `40:5,80:10` (from so many whole reais, so many percent
    /// more), or `none`. Refused unless each tier starts higher and gives more
    /// than the one before, and none gives more than 100%.
    pub fn parse(text: &str) -> Option<Self> {
        if text.trim() == "none" {
            return Some(Self::none());
        }
        let mut tiers: Vec<Tier> = Vec::new();
        for part in text.split(',') {
            let (reais, percent) = part.trim().split_once(':')?;
            let whole = |text: &str| {
                let text = text.trim();
                (!text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
                    .then(|| text.parse::<i64>().ok())
                    .flatten()
            };
            let tier = Tier {
                from_centavos: whole(reais)?.checked_mul(100)?,
                percent: whole(percent)?,
            };
            let rises = tiers.last().is_none_or(|last| {
                tier.from_centavos > last.from_centavos && tier.percent > last.percent
            });
            if !rises || !(1..=100).contains(&tier.percent) {
                return None;
            }
            tiers.push(tier);
        }
        Some(Self(tiers))
    }

    /// The tiers, lowest first.
    pub fn tiers(&self) -> &[Tier] {
        &self.0
    }

    /// The percentage a purchase of `centavos` adds: its highest tier's, or 0.
    pub fn percent(&self, centavos: i64) -> i64 {
        self.0
            .iter()
            .rev()
            .find(|tier| centavos >= tier.from_centavos)
            .map_or(0, |tier| tier.percent)
    }

    /// The bonus on `credit_micros` bought with `centavos`, floored.
    pub fn micros(&self, centavos: i64, credit_micros: i64) -> i64 {
        let added = i128::from(credit_micros) * i128::from(self.percent(centavos)) / 100;
        i64::try_from(added).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_tier_starts_at_its_exact_amount() {
        let bonus = Bonus::default();
        assert_eq!(bonus.percent(2_000), 0);
        assert_eq!(bonus.percent(3_999), 0);
        assert_eq!(bonus.percent(4_000), 5);
        assert_eq!(bonus.percent(7_999), 5);
        assert_eq!(bonus.percent(8_000), 10);
        assert_eq!(bonus.percent(500_000), 10);
    }

    #[test]
    fn the_bonus_is_a_floored_share_of_the_credits() {
        let bonus = Bonus::default();
        assert_eq!(bonus.micros(3_999, 8_000_000), 0);
        assert_eq!(bonus.micros(4_000, 8_016_032), 400_801);
        assert_eq!(bonus.micros(8_000, 16_032_064), 1_603_206);
    }

    #[test]
    fn tiers_parse_from_reais_and_percent() {
        assert_eq!(Bonus::parse("40:5,80:10"), Some(Bonus::default()));
        assert_eq!(Bonus::parse(" 40 : 5 , 80 : 10 "), Some(Bonus::default()));
        assert_eq!(Bonus::parse("none"), Some(Bonus::none()));
        assert_eq!(Bonus::none().micros(8_000, 1_000_000), 0);
    }

    #[test]
    fn the_environment_names_the_tiers_or_leaves_the_default() {
        let read = |pairs: Vec<(&str, &str)>| Bonus::from_environment(&Environment::of(pairs));
        assert_eq!(read(vec![]).unwrap(), Bonus::default());
        assert_eq!(read(vec![(RECHARGE_BONUS, "none")]).unwrap(), Bonus::none());
        let one = read(vec![(RECHARGE_BONUS, "100:15")]).unwrap();
        assert_eq!(one.percent(10_000), 15);
        assert!(matches!(
            read(vec![(RECHARGE_BONUS, "80:10,40:5")]),
            Err(ConfigError::Bonus { .. })
        ));
    }

    #[test]
    fn tiers_that_do_not_rise_or_do_not_read_are_refused() {
        for text in [
            "80:10,40:5",
            "40:10,80:5",
            "40:5,40:10",
            "40:0",
            "40:101",
            "40.5:5",
            "40",
            "40:5,",
            "-40:5",
            "",
        ] {
            assert_eq!(Bonus::parse(text), None, "{text:?}");
        }
    }
}
