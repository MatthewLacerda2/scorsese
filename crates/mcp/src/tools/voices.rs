//! Where a narration's `voice_id` comes from, without a window.
//!
//! An assistant assembling a narrated cut has to put *something* in `voice_id`,
//! and the one thing it must not do is guess — a plausible-looking id is either
//! a refused generation or, worse, somebody else's voice reading the film.
//! **Every ElevenLabs default voice expires on 2026-12-31**, so there is no id
//! this repository could carry that would still be right; the list is resolved
//! at runtime, and this is how a client reads it.
//!
//! Two questions at different sizes, and the second is the one with the date on
//! it: *which voices are there*, and *is the one already written down still
//! there*. A client mid-edit asks the second far more often than the first.

use scorsese_providers::credentials::{Provider, resolve};
use scorsese_providers::voices::{
    Answer, Catalogue, ElevenLabsVoices, Filters, VoiceError, builtin, library, require,
};
use serde_json::Value;

use crate::tools::{Costs, Reply, Tool, project_dir, project_property};

/// Listing the voices a narration can be read in.
pub(crate) struct Voices;

impl Tool for Voices {
    fn name(&self) -> &'static str {
        "voices"
    }

    fn description(&self) -> &'static str {
        "List the ElevenLabs voices a narration can be read in, or check that one \
         still exists. Free — a listing costs no credits — and cached inside the \
         project, so browsing needs no round trip and an offline call still \
         answers. There is no default voice and never will be: every ElevenLabs \
         default voice expires on 2026-12-31, so a voice id has to be chosen from \
         a list read today rather than remembered. Pass library to search the \
         Voice Library instead of the built-in set, narrowed by language — which \
         is what surfaces people who actually speak a language rather than read \
         it. Pass check with a voice id to ask about exactly that one; a voice \
         that has gone is reported as such, and nothing is ever silently \
         substituted."
    }

    fn costs(&self) -> Costs {
        Costs::Request
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "library": {
                    "type": "boolean",
                    "description": "Search the Voice Library — thousands of voices other \
                                    people published — instead of the small built-in set. \
                                    Needs a paid ElevenLabs plan; the built-in voices do \
                                    not. The filters below apply only to this."
                },
                "language": {
                    "type": "string",
                    "description": "Only voices in this language, as an ISO 639-1 code: pt, \
                                    en, es. The filter worth reaching for first — it is what \
                                    surfaces speakers of a language rather than readers of it."
                },
                "locale": {
                    "type": "string",
                    "description": "Only voices in this regional variant, where the vendor \
                                    has one: pt-br, en-us. Finer than language, and not \
                                    every voice carries it."
                },
                "gender": {
                    "type": "string",
                    "description": "Only voices of this gender: male, female, neutral."
                },
                "age": {
                    "type": "string",
                    "description": "Only voices of this age: young, middle_aged, old."
                },
                "accent": {
                    "type": "string",
                    "description": "Only voices with this accent, matched loosely: british, \
                                    brazilian."
                },
                "page_size": {
                    "type": "integer",
                    "description": "How many Voice Library voices to return, at most 100. \
                                    The vendor's own default is 30."
                },
                "refresh": {
                    "type": "boolean",
                    "description": "Read the list from ElevenLabs again even if the cached \
                                    one is still current. Rarely needed — the cache re-reads \
                                    itself weekly on its own."
                },
                "check": {
                    "type": "string",
                    "description": "A voice id to ask about instead of listing anything: \
                                    whether it still exists and can be generated with. Never \
                                    answered from the cache, because catching an id that has \
                                    gone is the whole point of asking."
                }
            },
            "required": ["project"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let key = resolve(Provider::ElevenLabs).map_err(|error| format!("{error}"))?;
        let catalogue = ElevenLabsVoices::new(&key.secret);

        if let Some(voice_id) = text(arguments, "check") {
            return checked(&catalogue, &voice_id);
        }

        let refresh = flag(arguments, "refresh");
        let from_library = flag(arguments, "library");
        let answer = if from_library {
            library(&dir, &catalogue, &filters(arguments)?, refresh)
        } else {
            builtin(&dir, &catalogue, refresh)
        };
        Ok(said(&answer.map_err(say)?, from_library).into())
    }
}

/// What one id's answer reads as.
///
/// A voice that has gone is reported as a refusal rather than as an empty
/// success, so a client cannot read past it: this is the call that stands
/// between a stale id and a generation that would be billed for reading the
/// film in somebody else's voice.
fn checked(catalogue: &dyn Catalogue, voice_id: &str) -> Result<Reply, String> {
    let voice = require(catalogue, voice_id).map_err(say)?;
    Ok(format!(
        "{}\n\nStill a voice at ElevenLabs, and can be generated with.",
        voice.says()
    )
    .into())
}

/// A failure as the sentence a client is shown.
///
/// Every one of these already carries its own advice — which permission to add,
/// which plan the Voice Library needs, that a missing voice has changed
/// nothing — so nothing is added here. Wrapping them would only put a second,
/// vaguer sentence in front of the useful one.
fn say(error: VoiceError) -> String {
    format!("{error}")
}

/// The Voice Library filters the arguments name.
fn filters(arguments: &Value) -> Result<Filters, String> {
    let page_size = match arguments.get("page_size") {
        None | Some(Value::Null) => None,
        Some(value) => Some(
            value
                .as_u64()
                .and_then(|size| u32::try_from(size).ok())
                .ok_or_else(|| format!("page_size: {value} is not a count of voices"))?,
        ),
    };
    Ok(Filters {
        language: text(arguments, "language"),
        locale: text(arguments, "locale"),
        gender: text(arguments, "gender"),
        age: text(arguments, "age"),
        accent: text(arguments, "accent"),
        page_size,
    })
}

/// A string argument, blank counting as absent — an empty filter is somebody
/// not filtering, and sending it would narrow a search to nothing.
fn text(arguments: &Value, name: &str) -> Option<String> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

/// A boolean argument, absent meaning false.
fn flag(arguments: &Value, name: &str) -> bool {
    arguments
        .get(name)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// The list, and where it came from.
///
/// The provenance line comes out of the library rather than being written here,
/// so that this and `scorsese voices` cannot drift into describing the same
/// cache differently.
fn said(answer: &Answer, from_library: bool) -> String {
    let what = if from_library {
        "the Voice Library"
    } else {
        "built-in"
    };
    let mut lines: Vec<String> = answer
        .voices
        .iter()
        .map(|voice| match voice.description.as_deref().map(str::trim) {
            Some(about) if !about.is_empty() => format!("{}\n    {about}", voice.says()),
            _ => voice.says(),
        })
        .collect();
    if lines.is_empty() {
        lines.push(format!("No voices matched in {what}."));
    }
    lines.push(format!("\n{}", answer.summary(what, "refresh: true")));
    lines.join("\n")
}
