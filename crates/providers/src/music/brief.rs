//! What a track asks for, resolved — and the fingerprint that keeps it from
//! being asked for twice.
//!
//! The sibling of [`speech::Brief`](crate::speech::Brief), held to the same
//! rule: the fingerprint is a compatibility surface, because the file a track
//! lands in is named for it. So the whole field set is written from the
//! start, the model and the format included though neither is a choice yet —
//! choosing one later then invalidates only what changed.

use scorsese_core::{
    Asset, AssetId, AudioBlock, MAX_MUSIC_SECONDS, MIN_MUSIC_SECONDS, ProjectPath,
};

use super::Incomplete;
use crate::api::elevenlabs::OUTPUT_FORMAT;
use crate::api::elevenlabs::music::MODEL_ID;
use crate::fingerprint::{self, Fingerprint};

/// Everything one track asks for, gathered and complete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Brief {
    /// The asset this is the brief of.
    pub id: AssetId,
    /// The description of the music.
    pub prompt: String,
    /// How long, in whole seconds — settled, so a `u32` here and an `Option`
    /// in the document.
    pub seconds: u32,
    /// Whether it is guaranteed to have no singing.
    pub instrumental: bool,
}

impl Brief {
    /// Gathers the brief of `asset`, or says what it is still missing —
    /// before a request is built, so the refusal names the asset and costs
    /// nothing.
    pub fn of(asset: &Asset) -> Result<Self, Incomplete> {
        let id = || asset.id.clone();
        let Some(AudioBlock::Music) = asset.audio_block() else {
            return Err(Incomplete::NotATrack { id: id() });
        };
        let request = asset.music.clone().unwrap_or_default();
        let prompt = asset
            .prompt
            .clone()
            .filter(|prompt| !prompt.trim().is_empty())
            .ok_or_else(|| Incomplete::NoPrompt { id: id() })?;
        let seconds = request
            .seconds
            .ok_or_else(|| Incomplete::NoLength { id: id() })?;
        if !(MIN_MUSIC_SECONDS..=MAX_MUSIC_SECONDS).contains(&seconds) {
            return Err(Incomplete::Length {
                id: id(),
                found: seconds,
                min: MIN_MUSIC_SECONDS,
                max: MAX_MUSIC_SECONDS,
            });
        }
        Ok(Self {
            id: id(),
            prompt,
            seconds,
            instrumental: request.instrumental,
        })
    }

    /// The fingerprint of everything this asks for.
    pub fn digest(&self) -> String {
        self.fingerprint().digest()
    }

    /// The text [`Brief::digest`] hashes: a labelled line per field.
    fn fingerprint(&self) -> Fingerprint {
        Fingerprint::of("elevenlabs-music")
            .line("model", MODEL_ID)
            .line("prompt", &self.prompt)
            .line("seconds", self.seconds)
            .line("instrumental", self.instrumental)
            .line("format", OUTPUT_FORMAT)
            // Reserved, so taking either later re-bills only what sets it: a
            // composition plan in place of the prompt, and a finetune.
            .line("composition_plan", "none")
            .line("finetune", "none")
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A track's fingerprint, pinned: if it moves, every track already paid
    /// for would be quoted and composed again.
    #[test]
    fn an_existing_briefs_fingerprint_does_not_move() {
        let brief = Brief {
            id: AssetId::new("bed"),
            prompt: String::from("Calm lo-fi."),
            seconds: 40,
            instrumental: true,
        };
        assert_eq!(brief.digest(), PINNED);
        assert_eq!(
            brief.output().as_str(),
            format!("generated/bed-{PINNED}.mp3")
        );
    }

    const PINNED: &str = "6c39fe322ed3b29dbe22e1e1651bec172cb3295e98b999bc6635bffd30e87afa";
}
