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

use scorsese_core::{
    Asset, AssetId, AssetKind, GENERATED_DIR, MAX_CHARACTERS, ProjectPath, SpeechRequest,
    hash_bytes,
};

use super::Incomplete;

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
        let text = asset
            .prompt
            .clone()
            .filter(|prompt| !prompt.trim().is_empty())
            .ok_or_else(|| Incomplete::NoPrompt {
                id: asset.id.clone(),
            })?;

        let characters = text.chars().count();
        if characters > MAX_CHARACTERS {
            return Err(Incomplete::TooLong {
                id: asset.id.clone(),
                found: characters,
                max: MAX_CHARACTERS,
            });
        }

        let request = asset.speech_request();
        let voice_id = request
            .voice_id
            .clone()
            .filter(|voice| !voice.trim().is_empty())
            .ok_or_else(|| Incomplete::NoVoice {
                id: asset.id.clone(),
            })?;

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
        hash_bytes(self.fingerprint().as_bytes())
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
    fn fingerprint(&self) -> String {
        let mut text = String::from("elevenlabs\n");
        text.push_str(&format!("model:{}\n", self.request.model.as_str()));
        text.push_str(&format!("voice:{}\n", self.voice_id));
        text.push_str(&format!("text:{}\n", self.text));
        text.push_str(&format!(
            "language:{}\n",
            or_none(self.request.language.as_deref())
        ));
        text.push_str(&format!(
            "seed:{}\n",
            self.request
                .seed
                .map_or_else(|| "none".to_owned(), |seed| seed.to_string())
        ));
        // Reserved. Each of these is a field scorsese may take later, and each
        // is written now so that taking it does not re-bill the projects that
        // never used it — see the module doc.
        text.push_str("format:mp3_44100_128\n");
        text.push_str("voice_settings:none\n");
        text.push_str("previous_text:none\n");
        text.push_str("next_text:none\n");
        text
    }

    /// Where a generation of this brief lands, project-relative.
    ///
    /// Named for the asset **and** the fingerprint, the same way a shot is: the
    /// fingerprint alone would let two assets share one file, and two lines
    /// that happen to say the same words in the same voice are still two takes.
    pub fn output(&self) -> ProjectPath {
        ProjectPath::new(format!("{GENERATED_DIR}/{}-{}.mp3", self.id, self.digest()))
    }

    /// Whether a generation of this brief is already on disk.
    ///
    /// The answer to *has this been paid for*, whatever the document claims
    /// about the asset — and the **current** brief's file, never the asset's
    /// recorded `path`, which after a rewrite still points at the previous
    /// reading. Every decision about whether a run has work goes through here
    /// so it cannot be made both ways.
    pub fn realized(&self, root: &std::path::Path) -> bool {
        self.output().resolve(root).is_file()
    }
}

/// A value, or the word that stands in for one that is not there.
///
/// `none` rather than an empty string, so a brief with no language and a brief
/// whose language was blank are different documents.
fn or_none(value: Option<&str>) -> &str {
    value.unwrap_or("none")
}
