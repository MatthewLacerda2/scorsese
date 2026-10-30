//! What was asked for, resolved — and the fingerprint that keeps it from being
//! asked for twice.
//!
//! The sibling of [`video::brief`](crate::video::brief), and simpler: a
//! narration names no files, so gathering one reads nothing off disk and the
//! only way it can be wrong is by being incomplete.
//!
//! # The money guarantee
//!
//! [`Brief::digest`] is a hash over every field of the request, and the file a
//! generation lands in is named for it. So an unchanged line is never billed
//! twice — which matters more here than it does for video, because a line of
//! narration is the thing in an edit people rewrite most.
//!
//! # The fingerprint is a compatibility surface
//!
//! This is the lesson Veo taught, written down where it applies. The hashed
//! text is a labelled line per field, and the generated file is named for the
//! result — so **adding a line later changes every existing digest and re-bills
//! every asset in every project**.
//!
//! The whole field set is therefore written from the start, including the
//! fields scorsese does not yet expose, each with `none` for a value. Adding
//! one later then invalidates only the assets that actually set it, instead of
//! all of them. `voice_settings` and the two continuity fields are here for
//! exactly that reason and for no other.
//!
//! Nothing about *when* is in it. A timestamp inside a brief hash would make an
//! unchanged line look edited and pay for it again.

use scorsese_core::{Asset, AssetId, AssetKind, ProjectPath, SpeechRequest, is_voice_id};

use super::Incomplete;
use crate::api::elevenlabs::OUTPUT_FORMAT;
use crate::fingerprint::{self, Fingerprint};

/// Everything one spoken line asks for, gathered and complete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Brief {
    /// The asset this is the brief of.
    pub id: AssetId,
    /// The words.
    pub text: String,
    /// The voice that says them. Resolved, so it is a `String` here and an
    /// `Option` in the document — this type is what exists once the question
    /// *has this been decided yet* is settled.
    pub voice_id: String,
    /// Model, language, seed.
    pub request: SpeechRequest,
}

impl Brief {
    /// Gathers the brief of `asset`, or says what it is still missing.
    ///
    /// Everything refused here is refused **before a request is built**, which
    /// is the difference between a message naming the asset and a charge for a
    /// reading nobody wanted.
    pub fn of(asset: &Asset) -> Result<Self, Incomplete> {
        if asset.kind != AssetKind::GeneratedAudio {
            return Err(Incomplete::NotGeneratedAudio {
                id: asset.id.clone(),
                kind: asset.kind,
            });
        }
        if asset.music.is_some() {
            return Err(Incomplete::Track {
                id: asset.id.clone(),
            });
        }
        let text = asset
            .prompt
            .clone()
            .filter(|prompt| !prompt.trim().is_empty())
            .ok_or_else(|| Incomplete::NoPrompt {
                id: asset.id.clone(),
            })?;

        let request = asset.speech_request();
        let characters = text.chars().count();
        let max = request.model.max_characters();
        if characters > max {
            return Err(Incomplete::TooLong {
                id: asset.id.clone(),
                found: characters,
                max,
            });
        }

        let voice_id = request
            .voice_id
            .clone()
            .filter(|voice| !voice.trim().is_empty())
            .ok_or_else(|| Incomplete::NoVoice {
                id: asset.id.clone(),
            })?;
        if !is_voice_id(&voice_id) {
            return Err(Incomplete::MalformedVoice {
                id: asset.id.clone(),
                voice: voice_id,
            });
        }

        Ok(Self {
            id: asset.id.clone(),
            text,
            voice_id,
            request,
        })
    }

    /// How many characters this is billed for.
    ///
    /// Characters and not bytes, which is the vendor's unit and also the only
    /// one that prices Portuguese correctly — its accented letters are two
    /// bytes each, so a byte count would overcharge a script by a quarter.
    pub fn characters(&self) -> usize {
        self.text.chars().count()
    }

    /// The fingerprint of everything this asks for.
    ///
    /// Written out as labelled lines rather than hashed field by field, so the
    /// hashed text is something a person can print and read when a cache
    /// behaves in a way nobody expects — and so two different briefs cannot
    /// collide by one field's value running into the next.
    pub fn digest(&self) -> String {
        self.fingerprint().digest()
    }

    /// The text [`Brief::digest`] hashes.
    ///
    /// The model goes in as the **document's** word for it rather than the
    /// vendor's id, matching what `video` does. The consequence is worth being
    /// deliberate about: if `expressive` is ever remapped to a newer vendor
    /// model, existing files stay valid rather than every narration in every
    /// project going stale and re-billing itself the day scorsese updated. That
    /// is the right default, and remapping deliberately is still available —
    /// it is simply not something an upgrade should do silently.
    fn fingerprint(&self) -> Fingerprint {
        Fingerprint::of("elevenlabs")
            .line("model", self.request.model.as_str())
            .line("voice", &self.voice_id)
            .line("text", &self.text)
            .or_none("language", self.request.language.as_deref())
            .or_none("seed", self.request.seed)
            // The format the vendor is asked for, and the one constant both
            // read: changing it re-bills every line, as it should, because the
            // file would be different audio.
            .line("format", OUTPUT_FORMAT)
            // Reserved. Each of these is a field scorsese may take later, and
            // each is written now so that taking it does not re-bill the
            // projects that never used it — see the module doc.
            .line("voice_settings", "none")
            .line("previous_text", "none")
            .line("next_text", "none")
    }

    /// Where a generation of this brief lands, project-relative — named for
    /// the asset **and** the fingerprint, the same way a shot is: two lines
    /// that happen to say the same words in the same voice are still two takes.
    pub fn output(&self) -> ProjectPath {
        fingerprint::output(&self.id, &self.digest(), "mp3")
    }

    /// Whether a generation of this brief is already on disk.
    ///
    /// The answer to *has this been paid for*, whatever the document claims
    /// about the asset — and the **current** brief's file, never the asset's
    /// recorded `path`, which after a rewrite still points at the previous
    /// reading. Every decision about whether a run has work goes through here
    /// so it cannot be made both ways.
    pub fn realized(&self, root: &std::path::Path) -> bool {
        self.output().resolve(root).is_ok_and(|file| file.is_file())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lines already spoken, pinned by their digests: one with every optional
    /// field absent and one with each set. If either moves, every narration
    /// already paid for would be quoted and paid for again.
    #[test]
    fn an_existing_briefs_fingerprint_does_not_move() {
        let mut brief = Brief {
            id: AssetId::new("line"),
            text: String::from("Welcome back."),
            voice_id: String::from("voice"),
            request: SpeechRequest::default(),
        };
        assert_eq!(
            brief.digest(),
            "bbd2456d8d5c2bc3f86fd35ff6e074de35594d4b71622a91e3581b5460737d60"
        );
        assert_eq!(
            brief.output().as_str(),
            "generated/line-bbd2456d8d5c2bc3f86fd35ff6e074de35594d4b71622a91e3581b5460737d60.mp3"
        );
        brief.request.language = Some(String::from("pt"));
        brief.request.seed = Some(7);
        assert_eq!(
            brief.digest(),
            "3ef26f5102a548a1c07c44cb54e9db81b6ec5bf392b7d6349358bc4632fb9d7a"
        );
    }
}
