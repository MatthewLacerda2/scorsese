//! ElevenLabs behind the trait: two listings in, voices out, refusals read.
//!
//! The whole of this file is translation, in both directions. What a call looks
//! like on the wire is [`api::elevenlabs`](crate::api::elevenlabs)'s business
//! and what to do with an answer is [`super`]'s; this is the one place that
//! turns the vendor's shapes into scorsese's, and it is a separate file for
//! exactly that reason — a translation spread through the code that makes the
//! call is a translation nobody can check against the vendor's documentation.
//!
//! **Reading a refusal is the interesting half.** A status code alone cannot
//! tell a key that is wrong from a key that is right but unscoped, nor a
//! withdrawn voice from one this account may not use, and those four call for
//! four different sentences. So each call maps the status *and* the vendor's
//! own `detail` onto the state it actually means, and everything unrecognised
//! stays a plain provider failure rather than being guessed at. *Sorting* a
//! refusal happens once for the whole of scorsese, in
//! [`Refusal`](crate::api::elevenlabs::refusal::Refusal), and reaches this file
//! already sorted through [`refusal`](super::refusal). What is left here is the
//! part that genuinely differs per endpoint: a plan limit means *the Voice
//! Library is not sold to this account* on a listing and *this voice may not be
//! used* on a lookup, and no reading of the body could tell those apart.

use crate::api::elevenlabs::voices::{self, Filters, Voices};
use crate::credentials::Secret;

use super::catalogue::{Availability, Catalogue, Voice};
use super::error::VoiceError;
use super::refusal::{NO_VOICE, failure, plan_limited, refused, unusable};

/// What this catalogue is called wherever a message names it.
const NAME: &str = "ElevenLabs";

/// The voice listings, as a catalogue.
#[derive(Debug, Clone)]
pub struct ElevenLabsVoices {
    voices: Voices,
}

impl ElevenLabsVoices {
    /// One that authenticates with this key.
    pub fn new(key: &Secret) -> Self {
        Self {
            voices: Voices::new(key),
        }
    }
}

impl Catalogue for ElevenLabsVoices {
    fn name(&self) -> &'static str {
        NAME
    }

    fn builtin(&self) -> Result<Vec<Voice>, VoiceError> {
        let listing = self
            .voices
            .premade()
            .map_err(|error| failure(NAME, error))?;
        Ok(listing.voices.iter().map(voice).collect())
    }

    fn library(&self, filters: &Filters) -> Result<Vec<Voice>, VoiceError> {
        let listing = self.voices.shared(filters).map_err(|error| {
            // The plan refusal only has a meaning on this endpoint: the Voice
            // Library is the part that is not sold to a free account, and the
            // built-in listing above is never refused for that reason.
            match refused(NO_VOICE, &error).as_ref().and_then(plan_limited) {
                Some(said) => VoiceError::NoLibraryOnThisPlan {
                    said: said.to_owned(),
                },
                None => failure(NAME, error),
            }
        })?;
        Ok(listing.voices.iter().map(voice).collect())
    }

    fn one(&self, voice_id: &str) -> Result<Availability, VoiceError> {
        match self.voices.one(voice_id) {
            Ok(found) => Ok(Availability::Available(voice(&found))),
            // A withdrawn voice and one this account may not use are both
            // answers about the voice — the first is the case with a date on
            // it. Everything else — a timeout, a 500, a key with no scope — is
            // a failure to *ask*, and says nothing about the voice. Turning one
            // of those into "unusable" is how a bad afternoon on the network
            // would look like a voice being withdrawn.
            Err(error) => match refused(voice_id, &error).as_ref().and_then(unusable) {
                Some(why) => Ok(Availability::Unusable(why)),
                None => Err(failure(NAME, error)),
            },
        }
    }
}

/// A vendor voice as scorsese carries one.
///
/// The two listings fill in different halves of the same facts — `/voices`
/// nests an accent and an age under `labels`, `/shared-voices` spreads them
/// across top-level fields — so both are read and whatever is present survives.
/// Flattening here rather than at each surface is what lets a built-in voice
/// and a Voice Library voice be printed in one list.
///
/// The order is the one somebody reads a voice in: where it is from, who is
/// speaking, what they sound like, and what it is for last.
fn voice(found: &voices::Voice) -> Voice {
    let mut traits: Vec<String> = Vec::new();
    let mut add = |value: &Option<String>| {
        if let Some(value) = value.as_ref().filter(|value| !value.trim().is_empty()) {
            traits.push(value.trim().to_owned());
        }
    };
    add(&found.language);
    add(&found.gender);
    add(&found.age);
    add(&found.accent);
    add(&found.descriptive);
    add(&found.use_case);
    // The labels come last and in the vendor's own order, which is alphabetical
    // by key: they are the built-in listing's way of saying the same things,
    // and a voice that carried both would otherwise say each of them twice.
    // Every label, including the one called `description`: on a built-in voice
    // that is a one-word descriptor — *casual*, *expressive* — and not the
    // sentence the top-level field of that name holds.
    for value in found.labels.values() {
        let value = value.trim();
        if !value.is_empty() && !traits.iter().any(|had| had.eq_ignore_ascii_case(value)) {
            traits.push(value.to_owned());
        }
    }
    Voice {
        id: found.voice_id.clone(),
        name: found.name.clone(),
        traits,
        description: found.description.clone(),
        preview: found.preview_url.clone(),
    }
}

/// The mapping above, read against what ElevenLabs actually sends.
///
/// `fixtures/elevenlabs/` holds two real response bodies — `/v1/voices?\
/// category=premade` and `/v1/shared-voices?language=pt`, both read on
/// 2026-08-05 — cut down to three voices each and scrubbed of the account ids
/// and signed preview URLs they arrived with. Nothing else about them was
/// changed, and that is the whole point: this mapping was written from the
/// vendor's reference page, and a field nested somewhere other than documented
/// deserialises to `None` and then prints as nothing at all rather than as
/// something wrong. Only a captured body can tell those two apart.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::elevenlabs::voices::Listing;

    /// The built-in listing, as the vendor sent it.
    const PREMADE: &str = include_str!("../../fixtures/elevenlabs/premade.json");

    /// The Voice Library listing, as the vendor sent it.
    const LIBRARY: &str = include_str!("../../fixtures/elevenlabs/shared-voices-pt.json");

    /// One captured body, parsed.
    fn listing(body: &str) -> Listing {
        serde_json::from_str(body).unwrap()
    }

    /// Every trait word in one vendor voice, from wherever its listing puts it.
    fn sent(found: &voices::Voice) -> Vec<String> {
        [
            &found.language,
            &found.gender,
            &found.age,
            &found.accent,
            &found.descriptive,
            &found.use_case,
        ]
        .into_iter()
        .flatten()
        .chain(found.labels.values())
        .cloned()
        .collect()
    }

    /// The difference the reference page describes is real: the built-in
    /// listing nests its traits under `labels` and fills in no top-level ones,
    /// the Voice Library listing does the exact opposite.
    #[test]
    fn the_two_listings_nest_their_traits_differently() {
        for found in &listing(PREMADE).voices {
            assert!(!found.labels.is_empty(), "{} has no labels", found.name);
            assert_eq!(found.language, None);
            assert_eq!(found.accent, None);
        }
        for found in &listing(LIBRARY).voices {
            assert!(found.labels.is_empty(), "{} has labels", found.name);
            assert!(found.language.is_some(), "{} has no language", found.name);
            assert!(found.accent.is_some(), "{} has no accent", found.name);
        }
    }

    /// A built-in voice reads its labels, in the vendor's own order — which is
    /// alphabetical by key, because that is what a `BTreeMap` iterates in.
    #[test]
    fn a_built_in_voice_reads_its_labels() {
        let voices: Vec<Voice> = listing(PREMADE).voices.iter().map(voice).collect();
        assert_eq!(voices[0].id, "CwhRBWXzGAHq8TQ4Fs17");
        assert_eq!(
            voices[0].traits,
            [
                "american",
                "middle_aged",
                "classy",
                "male",
                "en",
                "conversational"
            ]
        );
    }

    /// A Voice Library voice reads its top-level fields — `descriptive` among
    /// them, which is the one this fixture was captured to catch. It is not on
    /// the vendor's list of top-level fields, it is sent for every voice, and
    /// reading it only out of `labels` left the whole listing without it.
    #[test]
    fn a_library_voice_reads_its_top_level_fields() {
        let voices: Vec<Voice> = listing(LIBRARY).voices.iter().map(voice).collect();
        assert_eq!(voices[0].id, "KgIElyO5YK92T207OxYh");
        assert_eq!(
            voices[0].traits,
            [
                "pt",
                "male",
                "middle_aged",
                "brazilian",
                "calm",
                "conversational"
            ]
        );
    }

    /// The general form of the bug, over both bodies: nothing the vendor filled
    /// in is dropped on the way to a [`Voice`]. Whichever listing grows a field
    /// next is covered by this without anybody adding a case for it.
    #[test]
    fn every_trait_the_vendor_sent_survives() {
        for body in [PREMADE, LIBRARY] {
            let listing = listing(body);
            assert!(!listing.voices.is_empty(), "captured body has no voices");
            for found in &listing.voices {
                let mapped = voice(found);
                for word in sent(found) {
                    assert!(
                        mapped
                            .traits
                            .iter()
                            .any(|had| had.eq_ignore_ascii_case(&word)),
                        "{} lost {word:?} — kept {:?}",
                        found.name,
                        mapped.traits
                    );
                }
            }
        }
    }
}
