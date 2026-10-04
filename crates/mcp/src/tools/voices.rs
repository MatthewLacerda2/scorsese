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

use schemars::JsonSchema;
use scorsese_providers::credentials::{Provider, resolve};
use scorsese_providers::voices::{
    Answer, Catalogue, ElevenLabsVoices, Filters, VoiceError, builtin, library, require,
};
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, ProjectDir};
use crate::tools::{Costs, Reply, Tool};

/// Listing the voices a narration can be read in.
pub(crate) struct Voices;

/// What `voices` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// Search the Voice Library — thousands of voices other people published —
    /// instead of the small built-in set. Needs a paid ElevenLabs plan; the
    /// built-in voices do not. The filters below apply only to this. A search
    /// that matched more voices than one reply can carry says so, and how many
    /// matched in all — so a truncated search is never mistaken for a complete
    /// one.
    #[serde(default)]
    library: bool,
    /// Only voices in this language, as an ISO 639-1 code: pt, en, es. The
    /// filter worth reaching for first — it is what surfaces speakers of a
    /// language rather than readers of it.
    language: Option<String>,
    /// Only voices in this regional variant, where the vendor has one: pt-BR,
    /// en-US. Finer than language, and not every voice carries it. Any casing
    /// is accepted.
    locale: Option<String>,
    /// Only voices of this gender: male, female, neutral.
    gender: Option<String>,
    /// Only voices of this age: young, middle_aged, old.
    age: Option<String>,
    /// Only voices with this accent, matched loosely: british, brazilian.
    accent: Option<String>,
    /// How many Voice Library voices to return, at most 100. The vendor's own
    /// default is 30. It caps the reply and not the search — thousands can
    /// match, and the listing reports how many did.
    page_size: Option<u32>,
    /// Read the list from ElevenLabs again even if the cached one is still
    /// current. Rarely needed — the cache re-reads itself weekly on its own.
    #[serde(default)]
    refresh: bool,
    /// A voice id to ask about instead of listing anything: whether it still
    /// exists and can be generated with. Never answered from the cache, because
    /// catching an id that has gone is the whole point of asking.
    check: Option<String>,
}

impl args::Arguments for Arguments {}

impl Arguments {
    /// The Voice Library filters the arguments name — a blank one counting as
    /// absent, since an empty filter is somebody not filtering, and sending it
    /// would narrow a search to nothing.
    fn filters(&self) -> Filters {
        let text = |given: &Option<String>| args::given(given.as_deref()).map(ToOwned::to_owned);
        Filters {
            language: text(&self.language),
            locale: text(&self.locale),
            gender: text(&self.gender),
            age: text(&self.age),
            accent: text(&self.accent),
            page_size: self.page_size,
        }
    }
}

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
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let key = resolve(Provider::ElevenLabs).map_err(|error| format!("{error}"))?;
        let catalogue = ElevenLabsVoices::new(&key.secret);

        if let Some(voice_id) = args::given(arguments.check.as_deref()) {
            return checked(&catalogue, voice_id);
        }

        let from_library = arguments.library;
        let answer = if from_library {
            library(dir, &catalogue, &arguments.filters(), arguments.refresh)
        } else {
            builtin(dir, &catalogue, arguments.refresh)
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
