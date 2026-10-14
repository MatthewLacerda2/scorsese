//! What a dollar costs in reais, and what reais buy in credits (#548).
//!
//! Credits are dollars (#703); users pay in reais. The conversion is made
//! once, when a payment is started, at the commercial rate a person would
//! find by searching for it — AwesomeAPI's `ask`, falling back to the Central
//! Bank's PTAX selling rate (`api::rates`) — and **never guessed**: when
//! neither answers, [`Rates::usd_brl`] fails and no payment is started.
//!
//! **No float touches money.** A rate is kept as the text it arrived as
//! (`"5.0096"`), which is what the server stores beside the payment, and the
//! arithmetic in [`Rate::micros_for`] is integer: the decimal's digits over
//! its power of ten, in `u128`, floored to the micro-dollar.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::api::rates;

mod latest;

pub use latest::{Latest, REFRESH_EVERY, USABLE_FOR};

/// Where a rate came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// AwesomeAPI's commercial USD-BRL `ask`.
    Awesome,
    /// The Central Bank's PTAX selling rate.
    Ptax,
}

impl Source {
    /// The name stored beside a payment.
    pub fn name(self) -> &'static str {
        match self {
            Self::Awesome => "awesomeapi",
            Self::Ptax => "bcb_ptax",
        }
    }
}

/// Reais per dollar, as a source sent it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rate {
    /// The decimal as received, e.g. `5.0096`.
    pub brl_per_usd: String,
    /// Where it came from.
    pub source: Source,
}

/// The fewest and most whole reais a dollar is believed to cost, inclusive at
/// both ends and compared on the exact decimal (`8.01` is out). A real
/// BRL/USD rate has sat near 5 for years, so a wider band only lets a garbage
/// figure sell credits at a wild price: a rate of `0.05` would sell a hundred
/// times the credits. A figure outside is a source answering nonsense, and is
/// refused as a rate that never came.
const PLAUSIBLE: std::ops::RangeInclusive<u128> = 2..=8;

/// The most decimal places a rate is read to.
const MAX_PLACES: u32 = 12;

impl Rate {
    /// `text` as a rate from `source`, if it is a plausible decimal.
    pub fn parse(text: &str, source: Source) -> Option<Self> {
        let rate = Self {
            brl_per_usd: text.trim().to_owned(),
            source,
        };
        let (digits, places) = rate.decimal()?;
        let scale = 10u128.pow(places);
        let (low, high) = (PLAUSIBLE.start() * scale, PLAUSIBLE.end() * scale);
        (low..=high).contains(&digits).then_some(rate)
    }

    /// The decimal's digits and how many of them are past the point.
    fn decimal(&self) -> Option<(u128, u32)> {
        let (whole, fraction) = self
            .brl_per_usd
            .split_once('.')
            .unwrap_or((&self.brl_per_usd, ""));
        let places = u32::try_from(fraction.len()).ok()?;
        let all_digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
        if whole.is_empty() || !all_digits(whole) || !all_digits(fraction) || places > MAX_PLACES {
            return None;
        }
        let digits: u128 = format!("{whole}{fraction}").parse().ok()?;
        (digits > 0).then_some((digits, places))
    }

    /// The credits `centavos` buy at this rate, in micro-dollars, floored:
    /// `R$ 50,00` at `4.99` is `$10.020040`.
    pub fn micros_for(&self, centavos: i64) -> Option<i64> {
        let (digits, places) = self.decimal()?;
        let centavos = u128::try_from(centavos).ok()?;
        // micros = reais / rate × 10⁶ = centavos × 10⁴ × 10^places / digits.
        let micros = centavos
            .checked_mul(10_000)?
            .checked_mul(10u128.pow(places))?
            / digits;
        i64::try_from(micros).ok()
    }
}

/// Why no rate could be had: each source's reason.
#[derive(Debug, Clone, thiserror::Error)]
#[error("no USD-BRL rate: AwesomeAPI {awesome}; PTAX {ptax}")]
pub struct FxError {
    /// Why AwesomeAPI gave none.
    pub awesome: String,
    /// Why the PTAX service gave none.
    pub ptax: String,
}

/// Where a rate is read: the two sources, or a test's fixed figure.
///
/// Blocking: one or two HTTPS calls, made off the async runtime by whoever
/// holds this.
pub trait Rates: Send + Sync {
    /// What a dollar costs in reais now.
    fn usd_brl(&self) -> Result<Rate, FxError>;
}

/// AwesomeAPI, then the PTAX of the last week.
#[derive(Debug, Clone, Copy, Default)]
pub struct Live;

impl Rates for Live {
    fn usd_brl(&self) -> Result<Rate, FxError> {
        let awesome = match rates::awesome_ask() {
            Ok(text) => match Rate::parse(&text, Source::Awesome) {
                Some(rate) => return Ok(rate),
                None => format!("answered an implausible rate {text:?}"),
            },
            Err(error) => error.to_string(),
        };
        let today = days_since_epoch();
        let ptax = match rates::ptax_selling(&bcb_date(today - 7), &bcb_date(today)) {
            Ok(Some(text)) => match Rate::parse(&text, Source::Ptax) {
                Some(rate) => return Ok(rate),
                None => format!("answered an implausible rate {text:?}"),
            },
            Ok(None) => "had no bulletin in the last week".to_owned(),
            Err(error) => error.to_string(),
        };
        Err(FxError { awesome, ptax })
    }
}

/// Days from 1970-01-01 to today, in UTC.
fn days_since_epoch() -> i64 {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    i64::try_from(seconds / 86_400).unwrap_or(0)
}

/// The day `days` after 1970-01-01 as the BCB writes it, `MM-DD-YYYY`.
///
/// Howard Hinnant's `civil_from_days`, as `prices::Checked` does it: a date
/// is all that is needed, and no calendar crate is.
fn bcb_date(days: i64) -> String {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted + 2) / 5 + 1;
    let month = if shifted < 10 {
        shifted + 3
    } else {
        shifted - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{month:02}-{day:02}-{year}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rate(text: &str) -> Rate {
        Rate::parse(text, Source::Awesome).unwrap()
    }

    #[test]
    fn reais_buy_credits_floored_to_the_micro_dollar() {
        // The issue's example: R$ 50,00 at 4.99 is $10.02 in credits.
        assert_eq!(rate("4.99").micros_for(5_000), Some(10_020_040));
        assert_eq!(rate("5").micros_for(2_000), Some(4_000_000));
        // 20 / 5.00960 = 3.992334… dollars, floored.
        assert_eq!(rate("5.00960").micros_for(2_000), Some(3_992_334));
        assert_eq!(rate("5.0096").micros_for(2_000), Some(3_992_334));
        assert_eq!(rate("5.0096").micros_for(-1), None);
    }

    #[test]
    fn a_rate_that_is_not_a_plausible_decimal_is_refused() {
        for text in [
            "",
            "abc",
            "5,01",
            "-5.01",
            "0",
            "0.0",
            "0.5",
            "1",
            "1.99",
            "8.0001",
            "8.01",
            "50",
            "51",
            "1e3",
            ".5",
            "5.0000000000001",
        ] {
            assert_eq!(Rate::parse(text, Source::Ptax), None, "{text:?}");
        }
        assert_eq!(rate(" 5.01 ").brl_per_usd, "5.01");
        for text in ["2", "2.00", "2.0000001", "5.0096", "8", "8.000"] {
            assert!(Rate::parse(text, Source::Ptax).is_some(), "{text:?}");
        }
    }

    #[test]
    fn the_bcb_writes_month_first() {
        assert_eq!(bcb_date(0), "01-01-1970");
        // 2026-10-14 is day 20_740 since the epoch.
        assert_eq!(bcb_date(20_740), "10-14-2026");
        assert_eq!(bcb_date(20_740 - 7), "10-07-2026");
        assert_eq!(bcb_date(11_016), "02-29-2000");
    }
}
