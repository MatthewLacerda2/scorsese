//! What one design costs, before anyone spends it.
//!
//! # The arithmetic, and the one thing that is odd about it
//!
//! ElevenLabs bills text-to-speech by the character of input, so the figure is
//! known **exactly before the call** — a pre-flight estimate, never a
//! reconciliation. Voice Design is billed the same way, over the passage the
//! previews read, and **once**: three samples come back and one passage is
//! charged. That is the sentence worth carrying away, because the obvious guess
//! is three times the price and the obvious guess is wrong.
//!
//! **Round up.** Unlike Veo, this does not land on whole cents, and an estimate
//! that undershoots is one that slips past the ceiling it was computed for.
//!
//! # This rate is a lodger, and it says so
//!
//! The rate below belongs in `prices::elevenlabs`, beside `prices::veo`, with
//! its own [`Checked`] date so it joins `scorsese prices` and the CI price
//! signal for free. That table is being written on the branch for the
//! ElevenLabs provider itself, which is where it has to live — a second table
//! landing here first would be two rate tables for one vendor, and the first
//! person to notice they disagree would be the person who had already paid.
//!
//! So this holds one constant, dated the same way, and the move is a deletion:
//! when the provider's table lands, this module calls into it and the constant
//! goes. Nothing else here changes, because everything else is the arithmetic
//! rather than the number.
//!
//! Every figure this produces is **our own calculation over a page somebody
//! copied, never a bill** — no ElevenLabs response reports what a call cost.
//! That is why what gets recorded is called an *estimate* wherever it is
//! written down.

use crate::prices::{Checked, dollars};

/// What a thousand characters of designed speech costs, in US cents.
///
/// The vendor's base character rate, which is what Voice Design is billed at.
/// Read off <https://elevenlabs.io/pricing> on the date below.
const CENTS_PER_1K: u64 = 10;

/// The day the figure above was last read off the vendor's page.
const CHECKED: Checked = Checked::on(2026, 8, 5);

/// What one design is expected to cost, and what that was worked out from.
///
/// Carries the length and the rate as well as the total, because an estimate
/// somebody is about to approve should be checkable without re-deriving it —
/// *430 characters at 10¢ per 1000 is 5¢* is an argument, and `5` on its own is
/// a demand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Estimate {
    /// The total, in US cents. **Our calculation, never a billed figure.**
    pub cents: u64,
    /// How many characters were priced — the passage, once.
    pub characters: usize,
    /// The rate used, in cents per thousand characters.
    pub cents_per_1k: u64,
    /// The day that rate was last confirmed against the vendor's page.
    pub checked: Checked,
}

impl Estimate {
    /// The estimate as the sentence a surface prints.
    ///
    /// Written here rather than at each surface so that the command line, the
    /// MCP tool and whatever comes next cannot drift into quoting the same
    /// design at different prices — and so the disclaimer travels with the
    /// number instead of being remembered separately.
    pub fn says(&self) -> String {
        format!(
            "{} for the design — {} characters of preview text at {}¢ per 1000, charged once for \
             all three candidates. Our arithmetic over the published rate as of {}, never a bill.",
            dollars(self.cents),
            self.characters,
            self.cents_per_1k,
            self.checked
        )
    }
}

/// What designing from this passage is expected to cost.
///
/// The one answer to that question: the surfaces print it, the ceiling is
/// checked against it, and it is what gets written into the ledger beside the
/// voice. Three callers, one calculation — a second one would be a second
/// answer, and the first person to notice they disagree would be the person who
/// had already paid.
pub fn estimate(passage: &str) -> Estimate {
    let characters = passage.chars().count();
    Estimate {
        // Rounded up, on integers throughout. A float here would be a rounding
        // error inside a ceiling, and a ceiling nobody can reason about.
        cents: (u64::try_from(characters).unwrap_or(u64::MAX) * CENTS_PER_1K).div_ceil(1000),
        characters,
        cents_per_1k: CENTS_PER_1K,
        checked: CHECKED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rounding up is the whole point: an estimate that undershoots is one that
    /// slips past the ceiling it was computed for.
    #[test]
    fn a_part_of_a_cent_is_a_whole_cent() {
        assert_eq!(estimate(&"a".repeat(100)).cents, 1, "100 chars is 1¢ of 10");
        assert_eq!(estimate(&"a".repeat(101)).cents, 2);
        assert_eq!(estimate(&"a".repeat(1000)).cents, 10);
    }

    /// The vendor counts what a person counts, and a Portuguese passage is
    /// shorter in characters than in UTF-8 bytes. Pricing bytes would overcharge
    /// exactly the audience this feature exists for.
    #[test]
    fn characters_are_counted_rather_than_bytes() {
        let passage = "ação".repeat(50);
        assert_eq!(estimate(&passage).characters, 200);
        assert!(passage.len() > 200, "the test needs multi-byte characters");
    }

    /// The figure and its disclaimer travel together, because the figure alone
    /// reads as a bill.
    #[test]
    fn the_sentence_says_it_is_an_estimate() {
        let said = estimate(&"a".repeat(200)).says();
        assert!(said.contains("$0.02"), "{said}");
        assert!(said.contains("never a bill"), "{said}");
        assert!(said.contains("charged once"), "{said}");
    }
}
