//! What a sound effect asks for, resolved — and the fingerprint that keeps
//! it from being asked for twice.
//!
//! The sibling of [`music::Brief`](crate::music::Brief), held to the same
//! rule: the fingerprint is a compatibility surface, because the file a sound
//! lands in is named for it. So the whole field set is written from the
//! start, the model, the format and the prompt influence included though none
//! is a choice yet — choosing one later then invalidates only what changed.

use scorsese_core::{
    Asset, AssetId, AudioBlock, MAX_SOUND_SECONDS, MIN_SOUND_SECONDS, ProjectPath, SoundRequest,
};

use super::Incomplete;
use crate::api::elevenlabs::OUTPUT_FORMAT;
use crate::api::elevenlabs::sound::MODEL_ID;
use crate::fingerprint::{self, Fingerprint};

/// Everything one sound asks for, gathered and complete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Brief {
    /// The asset this is the brief of.
    pub id: AssetId,
    /// The description of the sound.
    pub prompt: String,
    /// How long, in whole milliseconds — the document's seconds, settled to
    /// a number the fingerprint and the price can both hold exactly.
    pub millis: u32,
    /// Whether it loops seamlessly.
    pub looping: bool,
}

impl Brief {
    /// Gathers the brief of `asset`, or says what it is still missing —
    /// before a request is built, so the refusal names the asset and costs
    /// nothing.
    pub fn of(asset: &Asset) -> Result<Self, Incomplete> {
        let id = || asset.id.clone();
        let Some(AudioBlock::Sound) = asset.audio_block() else {
            return Err(Incomplete::NotASound { id: id() });
        };
        let request = asset.sound.clone().unwrap_or_default();
        let prompt = asset
            .prompt
            .clone()
            .filter(|prompt| !prompt.trim().is_empty())
            .ok_or_else(|| Incomplete::NoPrompt { id: id() })?;
        let seconds = request
            .seconds
            .ok_or_else(|| Incomplete::NoLength { id: id() })?;
        if !SoundRequest::takes(seconds) {
            return Err(Incomplete::Length {
                id: id(),
                found: seconds,
                min: MIN_SOUND_SECONDS,
                max: MAX_SOUND_SECONDS,
            });
        }
        Ok(Self {
            id: id(),
            prompt,
            millis: millis_of(seconds),
            looping: request.looping,
        })
    }

    /// The fingerprint of everything this asks for.
    pub fn digest(&self) -> String {
        self.fingerprint().digest()
    }

    /// The text [`Brief::digest`] hashes: a labelled line per field.
    fn fingerprint(&self) -> Fingerprint {
        Fingerprint::of("elevenlabs-sound")
            .line("model", MODEL_ID)
            .line("prompt", &self.prompt)
            .line("millis", self.millis)
            .line("loop", self.looping)
            .line("format", OUTPUT_FORMAT)
            // Reserved, so taking it later re-bills only what sets it.
            .line("prompt_influence", "default")
    }

    /// Where a generation of this brief lands, project-relative — named for
    /// the asset and the fingerprint, like every prompted kind's.
    pub fn output(&self) -> ProjectPath {
        fingerprint::output(&self.id, &self.digest(), "mp3")
    }

    /// Whether a generation of this brief is already on disk — the answer to
    /// *has this been paid for*, and the current brief's file, never the
    /// asset's recorded `path`.
    pub fn realized(&self, root: &std::path::Path) -> bool {
        self.output().resolve(root).is_ok_and(|file| file.is_file())
    }
}

/// `seconds` in whole milliseconds. Only ever called on a length
/// [`SoundRequest::takes`], so the cast cannot overflow or go negative.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn millis_of(seconds: f64) -> u32 {
    (seconds * 1000.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sound's fingerprint, pinned: if it moves, every sound already paid
    /// for would be quoted and made again.
    #[test]
    fn an_existing_briefs_fingerprint_does_not_move() {
        let brief = Brief {
            id: AssetId::new("door"),
            prompt: String::from("A creaking door."),
            millis: 2_500,
            looping: false,
        };
        assert_eq!(brief.digest(), PINNED);
        assert_eq!(
            brief.output().as_str(),
            format!("generated/door-{PINNED}.mp3")
        );
    }

    #[test]
    fn seconds_settle_to_whole_milliseconds() {
        assert_eq!(millis_of(0.5), 500);
        assert_eq!(millis_of(2.5), 2_500);
        assert_eq!(millis_of(1.0 / 3.0), 333);
        assert_eq!(millis_of(30.0), 30_000);
    }

    const PINNED: &str = "df882bc05b0c23fbc270f2b02fe475b47eeb39b3cac2c09b74e5295f061a86d6";
}
