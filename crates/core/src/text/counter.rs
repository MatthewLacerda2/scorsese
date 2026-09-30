//! A number inside a text asset, written as a figure and free to count.
//!
//! The text carries a placeholder — [`PLACEHOLDER`], `{n}` — where the figure
//! goes, and the `number` block says how the figure is written: how many
//! decimals, which locale's separators, whether thousands are grouped. What the
//! figure *is* comes from the block's `value`, or from a `number` keyframe track
//! on the clip, which is how "144 partitions" counts up from nothing as it
//! lands. Everything around the placeholder is ordinary text, so a prefix and a
//! suffix are just words: `R$ {n} mi`, `{n}%`, `{n} partitions`.
//!
//! **The locale is the document's, never the machine's.** A render has to look
//! the same on every computer, so `1.234` versus `1,234` is a field somebody
//! chose rather than a setting read off whoever happened to render it.

use serde::{Deserialize, Serialize};

/// Where the figure goes in the text. Every occurrence gets the same figure.
pub const PLACEHOLDER: &str = "{n}";

/// The most decimals a figure may carry. Six is past anything a viewer reads
/// off a screen, and past it a double's own error starts to show in the digits.
pub const MAX_DECIMALS: u8 = 6;

/// Whose conventions a figure is written in: which mark groups the thousands
/// and which one sets off the decimals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Locale {
    /// English: `1,234.5`.
    #[default]
    #[serde(rename = "en")]
    En,
    /// Brazilian Portuguese: `1.234,5`.
    #[serde(rename = "pt-BR")]
    PtBr,
}

impl Locale {
    /// The mark between groups of three digits.
    const fn thousands(self) -> char {
        match self {
            Self::En => ',',
            Self::PtBr => '.',
        }
    }

    /// The mark before the decimals.
    const fn decimal(self) -> char {
        match self {
            Self::En => '.',
            Self::PtBr => ',',
        }
    }
}

/// How the figure in a text asset is written, and what it is when nothing
/// animates it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Counter {
    /// The figure shown when no `number` keyframe track animates it — usually
    /// the one the count ends on.
    pub value: f64,
    /// How many digits after the decimal mark, `0` to [`MAX_DECIMALS`]. The
    /// figure is rounded to them, so a count never shows a digit it is not
    /// going to keep.
    pub decimals: u8,
    /// Whose separators the figure is written with.
    pub locale: Locale,
    /// Whether thousands are grouped: `12.345` rather than `12345`. Off is what
    /// a year wants.
    pub grouping: bool,
}

impl Default for Counter {
    fn default() -> Self {
        Self {
            value: 0.0,
            decimals: 0,
            locale: Locale::default(),
            grouping: true,
        }
    }
}

impl Counter {
    /// `value` written the way this block says.
    ///
    /// Rounded half away from zero at the last decimal kept, grouped from the
    /// decimal mark leftwards, and a minus only when something non-zero is
    /// left after rounding — `-0.001` at no decimals is `0`, not `-0`.
    pub fn format(&self, value: f64) -> String {
        let decimals = usize::from(self.decimals.min(MAX_DECIMALS));
        let scale = 10f64.powi(decimals as i32);
        let rounded = (value.abs() * scale).round() / scale;
        let fixed = format!("{rounded:.decimals$}");
        let (whole, fraction) = fixed.split_once('.').unwrap_or((&fixed, ""));
        let mut out = String::new();
        if value < 0.0 && fixed.bytes().any(|digit| matches!(digit, b'1'..=b'9')) {
            out.push('-');
        }
        for (at, digit) in whole.chars().enumerate() {
            let left = whole.len() - at;
            if self.grouping && at > 0 && left % 3 == 0 {
                out.push(self.locale.thousands());
            }
            out.push(digit);
        }
        if !fraction.is_empty() {
            out.push(self.locale.decimal());
            out.push_str(fraction);
        }
        out
    }

    /// `text` with every placeholder replaced by `figure`.
    pub fn fill(text: &str, figure: &str) -> String {
        text.replace(PLACEHOLDER, figure)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counter(decimals: u8, locale: Locale) -> Counter {
        Counter {
            decimals,
            locale,
            ..Counter::default()
        }
    }

    #[test]
    fn the_locale_decides_which_mark_is_which() {
        assert_eq!(counter(1, Locale::En).format(1234.5), "1,234.5");
        assert_eq!(counter(1, Locale::PtBr).format(1234.5), "1.234,5");
    }

    #[test]
    fn thousands_are_grouped_from_the_decimal_mark_leftwards() {
        let en = counter(0, Locale::En);
        assert_eq!(en.format(0.0), "0");
        assert_eq!(en.format(999.0), "999");
        assert_eq!(en.format(1000.0), "1,000");
        assert_eq!(en.format(1_234_567.0), "1,234,567");
        let ungrouped = Counter {
            grouping: false,
            ..en
        };
        assert_eq!(ungrouped.format(2026.0), "2026");
    }

    #[test]
    fn rounding_is_half_away_from_zero_and_never_leaves_a_minus_on_nothing() {
        let en = counter(0, Locale::En);
        assert_eq!(en.format(143.5), "144");
        assert_eq!(en.format(-2.5), "-3");
        assert_eq!(en.format(-0.2), "0");
        assert_eq!(counter(2, Locale::PtBr).format(-1234.567), "-1.234,57");
    }

    #[test]
    fn every_placeholder_takes_the_figure() {
        assert_eq!(
            Counter::fill("{n} of {n} partitions", "144"),
            "144 of 144 partitions"
        );
    }
}
