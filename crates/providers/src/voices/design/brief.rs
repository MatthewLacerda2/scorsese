//! What one design asks for, and the fingerprint that keeps it from being paid
//! for twice.
//!
//! A design brief is small — a description, a passage to read, and two knobs —
//! which makes it tempting to pass around as four arguments. It is a type
//! because of the two things it does beyond holding them: it **refuses** a
//! request the vendor would refuse, before a round trip and before any money;
//! and it **hashes**, so that the same design asked for twice is answered from
//! disk rather than billed again.
//!
//! # The fingerprint is a compatibility surface
//!
//! The same lesson the video brief records, and it costs real money to get
//! wrong: the samples are stored under a name derived from this hash, so
//! **adding a line later changes every existing digest** and re-bills every
//! design anybody has already paid for. So the whole field set is written from
//! the start, each absent one spelled `none`, and a field added later
//! invalidates only the designs that actually set it.
//!
//! Nothing about *when* is in it. A timestamp inside a brief hash makes an
//! unchanged description look edited, and pay for it again.

use scorsese_core::hash_bytes;

use crate::api::elevenlabs::design::{DesignRequest, OUTPUT_FORMAT, PASSAGE, PROMPT};

use super::error::DesignError;

/// One design, as somebody asks for it.
#[derive(Debug, Clone, PartialEq)]
pub struct Brief {
    /// What the voice should be like, in a sentence.
    pub prompt: String,
    /// What the candidates read aloud, and the only thing billed.
    pub passage: String,
    /// The vendor's best-effort determinism knob.
    ///
    /// **Recorded even though it is only best-effort**, because it is half of
    /// what makes a lost voice worth attempting again: the description says
    /// what was asked for and the seed is the nearest thing there is to asking
    /// for it the same way twice.
    pub seed: Option<u32>,
    /// How literally the candidates should follow the description.
    pub guidance: Option<f64>,
}

impl Brief {
    /// A brief, refused here if the vendor would refuse it there.
    ///
    /// **Characters, not bytes.** The vendor counts what a person would count,
    /// and a description in Portuguese is shorter in characters than in UTF-8
    /// bytes — measuring bytes would refuse a valid sentence and blame the
    /// writer for the encoding.
    pub fn new(
        prompt: &str,
        passage: &str,
        seed: Option<u32>,
        guidance: Option<f64>,
    ) -> Result<Self, DesignError> {
        let prompt = prompt.trim().to_owned();
        let passage = passage.trim().to_owned();
        let counted = prompt.chars().count();
        if !PROMPT.contains(&counted) {
            return Err(DesignError::PromptLength {
                characters: counted,
            });
        }
        let counted = passage.chars().count();
        if !PASSAGE.contains(&counted) {
            return Err(DesignError::PassageLength {
                characters: counted,
            });
        }
        Ok(Self {
            prompt,
            passage,
            seed,
            guidance,
        })
    }

    /// The fingerprint of everything this asks for.
    ///
    /// Written out as labelled lines rather than hashed field by field, so the
    /// hashed text is something a person can print and read when the cache
    /// behaves in a way nobody expects — and so two different briefs cannot
    /// collide by one field's value running into the next.
    pub fn digest(&self) -> String {
        hash_bytes(self.fingerprint().as_bytes())
    }

    /// The text [`Brief::digest`] hashes.
    fn fingerprint(&self) -> String {
        let mut text = String::from("voice-design\n");
        text.push_str(&format!("prompt:{}\n", self.prompt));
        text.push_str(&format!("passage:{}\n", self.passage));
        text.push_str(&format!("seed:{}\n", said(self.seed)));
        text.push_str(&format!("guidance:{}\n", said(self.guidance)));
        text.push_str(&format!("format:{OUTPUT_FORMAT}\n"));
        // Written now and never set, for the reason in the module doc: the two
        // knobs the endpoint has that scorsese does not expose. Reserving the
        // lines costs nothing today and is what stops exposing one tomorrow
        // from re-billing every design already paid for.
        text.push_str("loudness:none\n");
        text.push_str("quality:none\n");
        text
    }

    /// This brief as the call the vendor takes.
    pub fn request(&self) -> DesignRequest {
        DesignRequest {
            voice_description: self.prompt.clone(),
            text: self.passage.clone(),
            output_format: OUTPUT_FORMAT,
            seed: self.seed,
            guidance_scale: self.guidance,
        }
    }
}

/// An optional value as a fingerprint line writes it.
///
/// `none` rather than an empty string, so a brief with no seed and a brief
/// whose seed was somehow blank are different documents.
fn said<T: std::fmt::Display>(value: Option<T>) -> String {
    value.map_or_else(|| String::from("none"), |value| value.to_string())
}
