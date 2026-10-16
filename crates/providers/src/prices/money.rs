//! Money as a person reads it, and as an operator writes it (#1287).
//!
//! **One formatter for every surface.** The CLI's tables, a quote, the hosted
//! server's assistant and its errors all print dollars through [`dollars`], so
//! a balance of $1,234 reads the same in a reply as on the web page
//! (`web/src/lib/money.ts` mirrors the rule, and its tests pin the same cases).
//! What differs between them is only how many places a figure is *at least*
//! shown with: two for an amount somebody pays, three for a rate copied from a
//! vendor's page (`$0.067`, as the page writes it).
//!
//! Integer arithmetic on the way out as well as in: `$0.05` is a division and
//! a remainder, not a float that prints as `0.05000000000000001`.

/// Micro-dollars in a US cent. The provider tables speak cents; the hosted
/// server's ledger speaks micro-dollars, and this is the one place they meet.
pub const MICROS_PER_CENT: u64 = 10_000;

/// Micro-dollars in a US dollar.
const MICROS_PER_DOLLAR: u128 = 1_000_000;

/// A provider table's cents, as micro-dollars.
pub const fn from_cents(cents: u64) -> u64 {
    cents.saturating_mul(MICROS_PER_CENT)
}

/// Micro-dollars as dollars: `$1.06`, `-$0.03`, `$1,234.00`.
///
/// At least `min_places` decimals, and as many more as the amount needs — up
/// to the micro-dollar — so `4_200` at two places is `$0.0042`, never a
/// misleading `$0.00`. The whole dollars carry a thousands separator.
pub fn dollars(micros: impl Into<i128>, min_places: usize) -> String {
    let micros: i128 = micros.into();
    let sign = if micros < 0 { "-" } else { "" };
    let magnitude = micros.unsigned_abs();
    let fraction = format!("{:06}", magnitude % MICROS_PER_DOLLAR);
    let places = fraction.trim_end_matches('0').len().max(min_places.min(6));
    let whole = grouped(magnitude / MICROS_PER_DOLLAR);
    format!("{sign}${whole}.{}", &fraction[..places])
}

/// `1234567` → `1,234,567`.
fn grouped(whole: u128) -> String {
    let digits = whole.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// Dollars as an operator writes them — `2`, `2.5`, `$2.50`, `$1,234.50` — as
/// micro-dollars. At most six places; `None` for anything else, a negative
/// amount included.
pub fn parse_dollars(text: &str) -> Option<i64> {
    let text = text.trim().trim_start_matches('$').replace(',', "");
    let (whole, fraction) = text.split_once('.').unwrap_or((&text, ""));
    let digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    if whole.is_empty() || !digits(whole) || !digits(fraction) || fraction.len() > 6 {
        return None;
    }
    let whole: i64 = whole.parse().ok()?;
    let fraction: i64 = format!("{fraction:0<6}").parse().ok()?;
    whole.checked_mul(1_000_000)?.checked_add(fraction)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cases `web/src/lib/money.test.ts` pins too, bar the minus sign:
    /// the page sets `−` (U+2212), a terminal and a log keep `-`.
    #[test]
    fn an_amount_reads_with_its_cents_and_no_fewer() {
        assert_eq!(dollars(0, 2), "$0.00");
        assert_eq!(dollars(1_060_000, 2), "$1.06");
        assert_eq!(dollars(2_500_000, 2), "$2.50");
        assert_eq!(dollars(4_200, 2), "$0.0042");
        assert_eq!(dollars(1, 2), "$0.000001");
        assert_eq!(dollars(1_234_567_890_000_i64, 2), "$1,234,567.89");
        assert_eq!(dollars(-2_500_000, 2), "-$2.50");
    }

    #[test]
    fn a_rate_reads_like_the_vendors_page() {
        assert_eq!(dollars(67_000_u64, 3), "$0.067");
        assert_eq!(dollars(33_600_u64, 3), "$0.0336");
        assert_eq!(dollars(1_500_000_u64, 3), "$1.500");
        assert_eq!(dollars(3_000_000_u64, 3), "$3.000");
        assert_eq!(dollars(15_000_000_u64, 3), "$15.000");
    }

    #[test]
    fn thousands_are_grouped() {
        assert_eq!(dollars(999_000_000, 2), "$999.00");
        assert_eq!(dollars(1_000_000_000, 2), "$1,000.00");
        assert_eq!(dollars(1_234_000_000, 2), "$1,234.00");
        assert_eq!(dollars(-1_234_000_000, 2), "-$1,234.00");
    }

    #[test]
    fn cents_are_ten_thousand_micros() {
        assert_eq!(from_cents(96), 960_000);
        assert_eq!(dollars(from_cents(96), 2), "$0.96");
        assert_eq!(dollars(from_cents(123_456), 2), "$1,234.56");
        assert_eq!(from_cents(u64::MAX), u64::MAX);
    }

    #[test]
    fn written_dollars_parse_to_micros() {
        for (written, micros) in [
            ("2", 2_000_000),
            ("2.5", 2_500_000),
            ("$2.50", 2_500_000),
            (" 3.50 ", 3_500_000),
            ("0.000001", 1),
            ("0", 0),
            ("$1,234.50", 1_234_500_000),
        ] {
            assert_eq!(parse_dollars(written), Some(micros), "{written}");
        }
        for nonsense in ["", "two", "-1", "1.2345678", ".5", "1.2.3", "$"] {
            assert_eq!(parse_dollars(nonsense), None, "{nonsense}");
        }
    }

    #[test]
    fn what_is_printed_parses_back() {
        for micros in [1, 4_200, 2_500_000, 1_234_567_890_000] {
            assert_eq!(parse_dollars(&dollars(micros, 2)), Some(micros));
        }
    }
}
