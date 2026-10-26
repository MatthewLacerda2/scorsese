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
use scorsese_providers::transcribe::{
    self, Asked, Hearing, LONGEST, Language, Undecoded, Whisper, correct, hear, keep,
};
use scorsese_render::audio::speech;
use scorsese_render::{Cancel, RenderError, Tools};
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, ProjectDir, Required};
use crate::tools::inspect::load;
use crate::tools::{Context, Costs, Reply, Tool};

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

    /// It changes only the transcript beside the asset, and that only once
    /// the server has answered: it holds the project for the write
    /// ([`Context::hold`]), never for the minutes it listens.
    fn writes(&self) -> bool {
        false
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        self.call_cancellable(arguments, &Cancel::new())
    }

    fn call_cancellable(&self, arguments: &Value, cancel: &Cancel) -> Result<Reply, String> {
        // Outside a session: no renders beside it, and nothing else to hold
        // the project against.
        let renders = crate::renders::Renders::default();
        self.call_in(arguments, &mut Context::new(cancel, &renders, None))
    }

    fn call_in(&self, arguments: &Value, context: &mut Context<'_>) -> Result<Reply, String> {
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
        let cancel = context.cancel;
        let hearing = Hearing {
            asked: &asked,
            transcriber: &Whisper::new(&address),
            decode: |file: &std::path::Path| {
                let tools =
                    Tools::discover().map_err(|error| Undecoded::Failed(error.to_string()))?;
                speech(&tools, file, LONGEST.as_secs_f64(), cancel).map_err(|error| match error {
                    RenderError::Stopped { .. } => Undecoded::Stopped,
                    other => Undecoded::Failed(other.to_string()),
                })
            },
            stop: &|| cancel.is_cancelled(),
        };
        let asset = AssetId::new(arguments.asset.as_str());
        let done = hear(&project, dir, &asset, hearing).map_err(|error| error.to_string())?;
        if done.heard.is_some() {
            // Read again under the hold: the project may have changed while
            // the server listened, and words are never written for a
            // recording that is gone.
            let _held = context.hold(&dir.to_string_lossy(), self.name())?;
            keep(&load(dir)?, dir, &done).map_err(|error| error.to_string())?;
        }
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use scorsese_core::words::{Word, Words};
    use scorsese_core::{Asset, AssetId, AssetKind, Fps, Project, ProjectPath};
    use serde_json::json;

    use super::*;
    use crate::renders::Renders;
    use crate::session::Holds;

    /// `transcribe` never waits behind a call changing its project: it
    /// listens unheld (#1549), and holds the project only to write.
    #[test]
    fn transcribing_does_not_wait_for_a_project_being_changed() {
        assert!(!Transcribe.writes());
        let dir = std::env::temp_dir().join(format!("scorsese-mcp-unheld-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("assets")).expect("as set up");
        let take = ProjectPath::new("assets/take.mp4");
        std::fs::write(take.resolve(&dir), b"footage").expect("as set up");
        let words = Words {
            words: vec![Word {
                text: "Corta!".into(),
                start: 0.0,
                end: 0.4,
            }],
        };
        std::fs::write(Words::transcript(&take).resolve(&dir), words.to_json()).expect("as set up");
        let mut project = Project::new("t", Fps::THIRTY);
        project.assets.push(Asset::imported(
            AssetId::new("take"),
            AssetKind::Video,
            take,
        ));
        project.save(&dir).expect("as set up");

        let holds = Holds::default();
        let project_dir = dir.to_string_lossy().into_owned();
        let _changing = holds.take(&project_dir, "clip_set").expect("as set up");
        let (cancel, renders) = (Cancel::new(), Renders::default());
        let mut context = Context::new(&cancel, &renders, None).holding(&holds);
        let started = std::time::Instant::now();
        let reply = Transcribe
            .call_in(&json!({ "project": dir, "asset": "take" }), &mut context)
            .expect("read back");
        assert!(started.elapsed() < Duration::from_secs(5));
        let text = &reply.parts[0].text;
        assert!(text.contains("was already transcribed"), "{text}");

        // What it holds to write, it holds until it is done.
        drop(_changing);
        let held = context.hold(&project_dir, "transcribe").expect("free");
        assert!(held.is_some());
        let outside = Context::new(&cancel, &renders, None);
        assert!(
            outside
                .hold(&project_dir, "transcribe")
                .expect("free")
                .is_none()
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
