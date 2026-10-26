//! An imported recording's speech as word timings, and a misheard word
//! corrected (#1549).
//!
//! `transcribe` asks the local Whisper server; `transcript_fix` is free and
//! offline. Both answer with the transcript numbered, which is what a
//! correction names a word by. The work is `scorsese_providers::transcribe`'s,
//! the decode `scorsese_render`'s; this is the wiring and the words.

use schemars::JsonSchema;
use scorsese_core::AssetId;
use scorsese_core::words::Fix;
use scorsese_providers::transcribe::{self, Asked, Language, Whisper, correct};
use scorsese_render::Tools;
use scorsese_render::audio::speech;
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, ProjectDir, Required};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool};

/// Hear an imported recording's words.
pub(crate) struct Transcribe;

/// Correct a transcript's words.
pub(crate) struct TranscriptFix;

/// What `transcribe` takes.
#[derive(Deserialize, JsonSchema)]
struct Heard {
    project: ProjectDir,
    /// Id of the imported video or sound whose speech to hear.
    asset: String,
    /// The language spoken, as a code like `pt` or `en`. Omit it and Whisper
    /// detects it; give it when the recording is short or mixes languages.
    language: Option<String>,
    /// Hear it afresh even though a transcript is kept, replacing it and every
    /// correction made to it. Default false: a kept transcript is read back.
    #[serde(default)]
    again: bool,
}

impl args::Arguments for Heard {
    const REQUIRED: Required = &[("asset", "the id of an imported video or sound")];
}

/// What `transcript_fix` takes.
#[derive(Deserialize, JsonSchema)]
struct Fixes {
    project: ProjectDir,
    /// Id of the transcribed video or sound.
    asset: String,
    /// The corrections, each naming a word by its number in the transcript as
    /// last shown.
    fixes: Vec<Correction>,
}

/// One correction.
#[derive(Deserialize, JsonSchema)]
struct Correction {
    /// The word's number, from 1, as the transcript was last shown.
    word: usize,
    /// What it says instead, punctuation included. Empty drops the word.
    text: String,
}

impl args::Arguments for Fixes {
    const REQUIRED: Required = &[
        ("asset", "the id of a transcribed video or sound"),
        ("fixes", "a list of {word, text} corrections"),
    ];
}

impl Tool for Transcribe {
    fn name(&self) -> &'static str {
        "transcribe"
    }

    fn description(&self) -> &'static str {
        "Hear the speech in an imported video or sound with a local Whisper, \
         keeping when each word is said. Then captions, pages and \
         project_describe work on real footage as on a generated line. The words are kept \
         beside the asset and shown numbered; correct a misheard one with \
         transcript_fix. Called again it reads the kept transcript back, \
         corrections included, unless `again` is set. Free, but it takes about a \
         minute of the computer for every half-minute of speech, and the Whisper \
         server must be running: when it is not, the error says how to start it."
    }

    fn costs(&self) -> Costs {
        Costs::Hearing
    }

    fn schema(&self) -> Value {
        args::schema::<Heard>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Heard = args::parse(arguments)?;
        let language = args::given(arguments.language.as_deref())
            .map(str::parse::<Language>)
            .transpose()
            .map_err(|error| error.to_string())?;
        let dir = arguments.project.dir();
        let project = load(dir)?;
        let address = transcribe::address().map_err(|error| error.to_string())?;
        let asked = Asked {
            language,
            again: arguments.again,
        };
        let done = transcribe::transcribe(
            &project,
            dir,
            &AssetId::new(arguments.asset.as_str()),
            &asked,
            &Whisper::new(&address),
            |file| {
                let tools = Tools::discover().map_err(|error| error.to_string())?;
                speech(&tools, file).map_err(|error| error.to_string())
            },
        )
        .map_err(|error| error.to_string())?;
        Ok(done.said().into())
    }
}

impl Tool for TranscriptFix {
    fn name(&self) -> &'static str {
        "transcript_fix"
    }

    fn description(&self) -> &'static str {
        "Correct misheard words in a transcript transcribe made, each keeping \
         when it is said, so captions timed to it stay in sync. Name each word by \
         its number in the transcript as last shown and give what it says \
         instead; an empty text drops a word heard where nothing was said. Every \
         number is the one shown before this call. Answers with the transcript \
         numbered afresh."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<Fixes>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Fixes = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let project = load(dir)?;
        let fixes: Vec<Fix> = arguments
            .fixes
            .into_iter()
            .map(|fix| Fix {
                word: fix.word,
                text: fix.text,
            })
            .collect();
        let corrected = correct(
            &project,
            dir,
            &AssetId::new(arguments.asset.as_str()),
            &fixes,
        )
        .map_err(|error| error.to_string())?;
        Ok(corrected.said().into())
    }
}
