//! What one design costs, before anyone spends it.
//!
//! # The arithmetic, and the one thing that is odd about it
//!
//! ElevenLabs bills by the character of input, so the figure is known
//! **exactly before the call** — a pre-flight estimate, never a
//! reconciliation. Voice Design is billed that same way, over the passage the
//! previews read, and **once**: three samples come back and one passage is
//! charged. That is the sentence worth carrying away, because the obvious guess
//! is three times the price and the obvious guess is wrong.
//!
//! # The rate is not this module's, and that is the point
//!
//! There is one ElevenLabs rate table in this crate — [`prices::elevenlabs`] —
//! and this reads it rather than carrying a figure of its own. Two tables for
//! one vendor is the arrangement where the first person to notice they disagree
//! is the person who has already paid, and it is also how a re-priced vendor
//! gets half-updated. So the whole of what lives here is *which* rate applies
//! and *what to say about it*: the standard character rate, which is what the
//! vendor bills a design at, and a sentence explaining the once-for-three.
//!
//! Rounding up happens there too, for the reason stated there: this does not
//! land on whole cents, and an estimate that undershoots is one that slips past
//! the ceiling it was computed for.
//!
//! Every figure here is **our own arithmetic over a page somebody copied, never
//! a bill** — no ElevenLabs response reports what a call cost. That is why what
//! gets recorded is called an *estimate* wherever it is written down.
//!
//! [`prices::elevenlabs`]: crate::prices::elevenlabs

use scorsese_core::SpeechModel;

use crate::prices::{self, SpeechEstimate, UnpricedSpeech, dollars};

/// Which rate a design is billed at.
///
/// The vendor's standard character rate. Voice Design runs on a model of its
/// own — a text-to-voice model rather than a text-to-speech one — but it is
/// **billed** at the base character price, and it is the billing this module is
/// about. Named here rather than left implicit, because it is the one
/// assumption in this file that a vendor could change without changing a
/// number.
const BILLED_AS: SpeechModel = SpeechModel::Standard;

/// What one design is expected to cost, and what that was worked out from.
///
/// The speech estimate, with the design's own way of saying it — an estimate
/// somebody is about to approve should be checkable without re-deriving it, and
/// *118 characters at 10¢ per 1000 is 2¢* is an argument where `2` on its own is
/// a demand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Estimate {
    /// The total, in US cents. **Our calculation, never a billed figure.**
    pub cents: u64,
    /// How many characters were priced — the passage, once.
    pub characters: usize,
    /// The rate it came from, date included.
    pub rate: prices::elevenlabs::Rate,
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
            self.rate.cents_per_1k_chars,
            self.rate.checked
        )
    }
}

impl From<SpeechEstimate> for Estimate {
    fn from(estimate: SpeechEstimate) -> Self {
        Self {
            cents: estimate.cents,
            characters: estimate.characters,
            rate: estimate.rate,
        }
    }
}

/// What designing from this passage is expected to cost.
///
/// The one answer to that question: the surfaces print it, the ceiling is
/// checked against it, and it is what gets written into the ledger beside the
/// voice. Three callers, one calculation — a second one would be a second
/// answer, and the first person to notice they disagree would be the person who
/// had already paid.
///
/// **Characters, not bytes**, because that is what the vendor counts. A
/// Portuguese passage is shorter in characters than in UTF-8 bytes, so pricing
/// bytes would overcharge exactly the audience this feature exists for.
pub fn estimate(passage: &str) -> Result<Estimate, UnpricedSpeech> {
    prices::speech(BILLED_AS, passage.chars().count()).map(Estimate::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rounding up is the whole point: an estimate that undershoots is one that
    /// slips past the ceiling it was computed for.
    #[test]
    fn a_part_of_a_cent_is_a_whole_cent() {
        let cents = |length: usize| estimate(&"a".repeat(length)).unwrap().cents;
        assert_eq!(cents(100), 1, "100 characters is 1¢ of the 10¢ per 1000");
        assert_eq!(cents(101), 2);
        assert_eq!(cents(1000), 10);
    }

    /// The vendor counts what a person counts, and a Portuguese passage is
    /// shorter in characters than in UTF-8 bytes.
    #[test]
    fn characters_are_counted_rather_than_bytes() {
        let passage = "ação".repeat(50);
        assert_eq!(estimate(&passage).unwrap().characters, 200);
        assert!(passage.len() > 200, "the test needs multi-byte characters");
    }

    /// The figure and its disclaimer travel together, because the figure alone
    /// reads as a bill.
    #[test]
    fn the_sentence_says_it_is_an_estimate() {
        let said = estimate(&"a".repeat(200)).unwrap().says();
        assert!(said.contains("$0.02"), "{said}");
        assert!(said.contains("never a bill"), "{said}");
        assert!(said.contains("charged once"), "{said}");
    }
}
