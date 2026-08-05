//! What designing a voice is, from scorsese's side of the line.
//!
//! **Nothing here is a client.** [`Studio`] is the trait everything above talks
//! to, which is what lets a feature that spends money at a vendor be driven end
//! to end by tests that spend nothing and need no key — the repo rule satisfied
//! by construction rather than by anyone remembering it.
//!
//! It is deliberately the *sibling* of [`Catalogue`](super::super::Catalogue)
//! rather than an extension of it. A catalogue **finds** a voice and costs
//! nothing; a studio **makes** one, spends money doing it, and leaves something
//! behind in an account. Two traits, because a caller holding one should be
//! unable to do the other by accident — and because a listing implementation
//! that had to stub out a billed method would be a listing that could bill.
//!
//! What they do share is the answer: a design ends in a
//! [`Voice`](super::super::Voice), the same type a listing yields, because a
//! designed voice is a voice. Anything else would leave two ways to say the
//! same thing and a surface that had to know which it was holding.

use super::brief::Brief;
use super::error::DesignError;

/// One candidate, as it came back.
///
/// Carries the bytes rather than a path: **who writes a file and where** is a
/// decision about the project directory, and a provider implementation should
/// not be making it. The fake in the tests is the proof — it produces
/// candidates without touching a disk at all.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    /// The vendor's handle for this one. **Not a `voice_id`**: nothing can be
    /// generated with it, and it lives only until the vendor's window closes.
    pub generated_voice_id: String,
    /// The sample, decoded — an MP3 of this candidate reading the passage.
    pub sample: Vec<u8>,
    /// How long it runs, where the vendor said.
    pub seconds: Option<f64>,
}

/// Somewhere a voice can be designed.
///
/// Two calls, matching the vendor's two, and the split is not an implementation
/// detail leaking through: the first spends money and the second does not, and
/// a person auditions three samples in between. Collapsing them into one
/// "design a voice" call would either spend money on a choice nobody had made
/// yet, or hide the audition from the only surface that can hold it.
pub trait Studio {
    /// Designs candidates for this brief. **The call that is billed.**
    fn candidates(&self, brief: &Brief) -> Result<Vec<Candidate>, DesignError>;

    /// Turns one candidate into a voice in the account, under `name`.
    ///
    /// `passed_over` are the candidates that were heard and not chosen; the
    /// vendor uses them to improve the model and sending them costs nothing.
    /// `brief` goes with them because the vendor keeps the description on the
    /// voice, which is what stops an account filling with voices nobody can
    /// account for.
    ///
    /// Answers with a [`Voice`](super::super::Voice) because that is what now
    /// exists — from here on it is a voice like any other, and the id it
    /// carries is the one a narration names.
    fn keep(
        &self,
        brief: &Brief,
        chosen: &str,
        name: &str,
        passed_over: &[String],
    ) -> Result<super::super::Voice, DesignError>;

    /// What this studio is called, for a message somebody reads.
    fn name(&self) -> &'static str;
}
