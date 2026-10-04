//! ElevenLabs' part of the live check: a listing, a word, a design.
//!
//! Three calls, each the cheapest that exercises a path scorsese depends on:
//!
//! - **`GET /v1/voices?category=premade`** — free. The key, the `voices_read`
//!   scope, and the listing shape. It also supplies the voice for the next
//!   call: an id written into this file would be an outage with a date on it,
//!   since every default voice expires on 2026-12-31.
//! - **text-to-speech of [`LINE`]** on the `fast` model — a cent, rounded up
//!   from a fraction of one. The body is an MP3 or it is a failure.
//! - **Voice Design of [`PASSAGE`]** — a cent: the vendor's hundred-character
//!   minimum, billed once for three candidates. Its reply has no captured
//!   fixture, so this is the only thing that checks it. The follow-up call
//!   that *keeps* a candidate is deliberately not made: it costs nothing but
//!   leaves a voice in the account on every run.

use base64::Engine;
use scorsese_core::SpeechModel;

use crate::api::elevenlabs::design::{
    CANDIDATES, Design, DesignReply, DesignRequest, OUTPUT_FORMAT,
};
use crate::api::elevenlabs::speech::{Speak, Speech};
use crate::api::elevenlabs::voices::{Listing, Voices};
use crate::api::http::HttpError;
use crate::api::tap::Tap;
use crate::credentials::Secret;
use crate::prices::{self, dollars};
use crate::voices::design as price;

use super::{Step, Verdict, judge};

/// What the speech call reads aloud.
pub const LINE: &str = "Checking.";

/// What the design call's candidates read: the vendor's minimum, a hundred
/// characters, because it is the number the call is billed by.
pub const PASSAGE: &str = "This is a short passage, read aloud just once, so that the check can hear the voice it has designed.";

/// The voice the design call asks for.
pub const DESCRIPTION: &str =
    "A calm, clear adult voice with a neutral accent, speaking at an even pace.";

/// The speech model the check speaks with: the cheapest.
const MODEL: SpeechModel = SpeechModel::Fast;

/// What the speech call costs, in cents.
fn speech_cents() -> u64 {
    prices::speech(MODEL, LINE.chars().count())
        .expect("every speech model has a published rate")
        .cents
}

/// What the design call costs, in cents.
fn design_cents() -> u64 {
    price::estimate(PASSAGE)
        .expect("the design rate is published")
        .cents
}

/// What the ElevenLabs part of the check costs, in cents.
pub fn cost() -> u64 {
    speech_cents() + design_cents()
}

/// The calls, as a quote lists them.
pub(super) fn calls() -> Vec<String> {
    vec![
        String::from("GET voices?category=premade — free"),
        format!(
            "speak {LINE:?} ({} characters, fast model) — {}",
            LINE.chars().count(),
            dollars(speech_cents())
        ),
        format!(
            "design a voice from a {}-character passage, keeping none of the three — {}",
            PASSAGE.chars().count(),
            dollars(design_cents())
        ),
    ]
}

/// Runs the ElevenLabs part; returns what it found and what it spent.
pub(super) fn check(key: &Secret, tap: &Tap) -> (Vec<Step>, u64) {
    let (listed, voice) = listing_step(Voices::new(key).tapped(tap).premade());
    let mut steps = vec![listed];
    let mut spent = 0;
    match voice {
        Some(voice) => {
            let body = Speak {
                text: LINE.to_owned(),
                model_id: MODEL.model_id().to_owned(),
                ..Speak::default()
            };
            let step = speech_step(Speech::new(key).tapped(tap).speak(&voice, &body));
            spent += paid(&step, speech_cents());
            steps.push(step);
        }
        None => steps.push(Step::new(
            "POST text-to-speech",
            Verdict::Skipped {
                why: String::from("no voice to speak with: the listing did not give one"),
            },
        )),
    }
    let request = DesignRequest {
        voice_description: DESCRIPTION.to_owned(),
        text: PASSAGE.to_owned(),
        output_format: OUTPUT_FORMAT,
        seed: None,
        guidance_scale: None,
    };
    let step = design_step(Design::new(key).tapped(tap).preview(&request));
    spent += paid(&step, design_cents());
    steps.push(step);
    (steps, spent)
}

/// What a step cost: the estimate if the vendor did the work, nothing if it
/// refused. A shape change still counts — the call succeeded, and was billed,
/// before we failed to read it.
fn paid(step: &Step, cents: u64) -> u64 {
    match step.verdict {
        Verdict::Ok | Verdict::ShapeChanged { .. } => cents,
        _ => 0,
    }
}

/// What the premade listing said, and a voice to speak with.
pub fn listing_step(answer: Result<Listing, HttpError>) -> (Step, Option<String>) {
    let call = "GET voices?category=premade";
    let listing = match answer {
        Err(error) => return (Step::new(call, judge::elevenlabs(&error)), None),
        Ok(listing) => listing,
    };
    let Some(first) = listing.voices.first() else {
        let field = String::from("voices: the premade listing came back empty");
        return (Step::new(call, Verdict::ShapeChanged { field }), None);
    };
    if listing
        .voices
        .iter()
        .any(|voice| voice.voice_id.is_empty() || voice.name.is_empty())
    {
        let field = String::from("voices[].voice_id / name: a voice arrived without one");
        return (Step::new(call, Verdict::ShapeChanged { field }), None);
    }
    let step =
        Step::new(call, Verdict::Ok).noting(format!("{} premade voices", listing.voices.len()));
    (step, Some(first.voice_id.clone()))
}

/// What the speech call gave back.
pub fn speech_step(answer: Result<Vec<u8>, HttpError>) -> Step {
    let call = "POST text-to-speech";
    match answer {
        Err(error) => Step::new(call, judge::elevenlabs(&error)),
        Ok(bytes) if judge::is_mp3(&bytes) => {
            Step::new(call, Verdict::Ok).noting(format!("{} bytes of MP3", bytes.len()))
        }
        Ok(bytes) => Step::new(
            call,
            Verdict::ShapeChanged {
                field: format!("body: expected an MP3, got {}", judge::looks_like(&bytes)),
            },
        ),
    }
}

/// What the design call gave back: three candidates, each an id and an MP3.
pub fn design_step(answer: Result<DesignReply, HttpError>) -> Step {
    let call = "POST text-to-voice/design";
    let reply = match answer {
        Err(error) => return Step::new(call, judge::elevenlabs(&error)),
        Ok(reply) => reply,
    };
    let field = if reply.previews.len() != CANDIDATES {
        Some(format!(
            "previews: expected {CANDIDATES}, got {}",
            reply.previews.len()
        ))
    } else if reply
        .previews
        .iter()
        .any(|p| p.generated_voice_id.is_empty())
    {
        Some(String::from("previews[].generated_voice_id: missing"))
    } else if reply.previews.iter().any(|p| {
        !base64::engine::general_purpose::STANDARD
            .decode(&p.audio_base_64)
            .is_ok_and(|audio| judge::is_mp3(&audio))
    }) {
        Some(String::from("previews[].audio_base_64: not a base64 MP3"))
    } else {
        None
    };
    match field {
        Some(field) => Step::new(call, Verdict::ShapeChanged { field }),
        None => Step::new(call, Verdict::Ok).noting(format!("{CANDIDATES} candidates, none kept")),
    }
}
