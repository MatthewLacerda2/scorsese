//! What a paid call would spend, and the token that lets it.
//!
//! # Two steps, because an agent has nobody to ask
//!
//! A person at a terminal is asked `[y/N]` before `scorsese generate` spends.
//! A tool call has no terminal: whatever is on the other end of it — an
//! assistant in a chat, the web editor's own assistant, somebody's client over
//! web MCP — decides by itself whether to spend, unless the tool makes that a
//! two-step exchange. So every tool that spends money answers its first call
//! with a [`Quote`] and an [`Issued`] token, and **only a second call carrying
//! that token spends**. What happens between the two — a confirmation box, a
//! question in a chat, nothing at all — is the client's business; the protocol
//! guarantees there *is* a between.
//!
//! # A token is bound to exactly what was quoted
//!
//! [`Quote::digest`] covers every brief that would be paid for — its asset,
//! the hash of the brief itself, and the cents it was priced at — and nothing
//! that would not be. A token is redeemed only against a fresh quote with the
//! same digest, so an edited prompt, a new sketch added in between, or a
//! re-priced rate table all need a new quote. A shot that finished or was
//! collected meanwhile does **not**: what it would have cost is no longer being
//! asked for, and nothing more is spent than was agreed.
//!
//! # Single use, short-lived, and stored
//!
//! A token is good for **one attempt** within [`LIFETIME_SECONDS`]. Any attempt
//! consumes it, including a refused one, because the answer to every refusal
//! is the same — quote again, which is free. It is also **stored** rather than
//! self-verifying: a token nobody issued is refused because it is not in the
//! store, not because a checksum failed, so there is no way to construct one.
//! The store is the [`Quotes`] trait. Locally it is [`ProjectQuotes`], a file
//! per token under the project's rebuildable `cache/`; the hosted server keeps
//! the same [`Issued`] record in its jobs and credits tables.
//!
//! # What this is not
//!
//! **Not the ceiling.** [`Budget`](crate::credentials::Budget) is still checked
//! inside the pass that submits, after the token is redeemed, so a confirmed
//! quote can still be refused there. One is permission from whoever is
//! watching; the other is a number that holds when nobody is.
//!
//! **Not a bill.** Every figure is our arithmetic over a copied rate table —
//! see [`prices`](crate::prices).

mod generation;
mod local;
mod token;

pub use generation::{Unquotable, generation};
pub use local::{ProjectQuotes, QUOTES_DIR};
pub use token::{Issued, LIFETIME_SECONDS, Quotes, Refused, issue, redeem};

use scorsese_core::hash_bytes;

/// What kind of spending a quote is for.
///
/// Part of the digest, so a token issued for one kind cannot be spent on the
/// other even in the unlikely case that the rest of the two digests agreed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Spend {
    /// Realising prompted assets: Veo shots and ElevenLabs lines.
    Generation,
    /// Designing a voice from a description.
    VoiceDesign,
}

impl Spend {
    /// How it is written in a digest and in a stored record.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Generation => "generation",
            Self::VoiceDesign => "voice_design",
        }
    }
}

/// One line of a quote: something the call would act on, and what it costs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// What it is about — an asset id, or `design` for a voice design.
    pub subject: String,
    /// What the call would do with it, in a sentence a surface prints after
    /// the subject. Carries the price when there is one.
    pub says: String,
    /// What it would be charged, when it would be charged at all. Absent for a
    /// brief already paid for, one still in flight, or one not ready to send.
    pub charge: Option<Charge>,
}

/// A brief that would be paid for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Charge {
    /// The hash of the brief — the same one its output is named after.
    pub brief: String,
    /// What it is estimated to cost, in US cents.
    pub cents: u64,
}

/// What one call would spend, item by item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quote {
    /// What kind of spending this is.
    pub spend: Spend,
    /// Every item the call would act on, charged or not, in document order.
    pub items: Vec<Item>,
}

impl Quote {
    /// The whole estimate, in US cents.
    pub fn cents(&self) -> u64 {
        self.charges().map(|(_, charge)| charge.cents).sum()
    }

    /// Whether nothing here would be paid for — the case that needs no token,
    /// because there is nothing to agree to.
    ///
    /// Asked of the charges and never of [`Quote::cents`]: a brief priced at
    /// nothing is still a brief handed to a vendor, and it still gets asked
    /// about.
    pub fn is_free(&self) -> bool {
        self.charges().next().is_none()
    }

    /// The fingerprint a token is bound to.
    ///
    /// Only the charged items go in, sorted, so the digest moves exactly when
    /// what would be paid for moves — and not when a shot already paid for is
    /// collected, or two assets swap places in the table. Labelled lines, for
    /// the reason every brief fingerprint here is written that way: one
    /// field's value can never run into the next.
    pub fn digest(&self) -> String {
        let mut charges: Vec<String> = self
            .charges()
            .map(|(subject, charge)| {
                format!(
                    "item:{subject}\nbrief:{}\ncents:{}\n",
                    charge.brief, charge.cents
                )
            })
            .collect();
        charges.sort();
        let text = format!("spend:{}\n{}", self.spend.as_str(), charges.concat());
        hash_bytes(text.as_bytes())
    }

    /// Every charged item, with what it is about.
    fn charges(&self) -> impl Iterator<Item = (&str, &Charge)> {
        self.items
            .iter()
            .filter_map(|item| Some((item.subject.as_str(), item.charge.as_ref()?)))
    }
}
