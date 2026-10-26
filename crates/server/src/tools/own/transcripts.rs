//! `transcribe` and `transcript_fix` on the web (#1549): a recording's words
//! heard by a job, and kept — corrections too — on its library item.
//!
//! Locally both work on a file beside the asset. A stored project's folder is
//! gone the moment a tool answers, so here the words live on the library row
//! ([`crate::library::transcribe`]) and every layout of the project writes
//! them beside the asset. What is refused, how a transcript reads and how a
//! correction is made are `scorsese_providers::transcribe`'s and
//! `scorsese_core::words`' — the same words the CLI and the stdio server say.

use scorsese_core::words::{Fix, Words};
use scorsese_core::{Asset, AssetId, ProjectPath};
use scorsese_mcp::Reply;
use scorsese_providers::transcribe::{Corrected, Language, Transcribed, recording};
use serde_json::{Value, json};

use super::super::surface::project_property;
use super::super::{Caller, database, project_id};
use crate::library::LibraryError;
use crate::library::transcribe::Payload;
use crate::projects::{self, ProjectError};

/// How a client names the transcription.
pub(super) const TRANSCRIBE: &str = "transcribe";

/// What it does.
pub(super) const TRANSCRIBE_SAYS: &str = "Hear the speech in an imported video or sound, \
keeping when each word is said. Then captions, pages and project_describe work on real footage \
as on a generated line. It runs as a free job, about a minute for every half-minute of speech: \
the reply names the job, and jobs with wait: true answers when it is done. Called again once \
it is, it answers with the words numbered — correct a misheard one with transcript_fix — and \
it reads that kept transcript back, corrections included, unless `again` is set. The words \
belong to the library file, so every project using it shares them.";

/// How a client names the correction.
pub(super) const FIX: &str = "transcript_fix";

/// What it does.
pub(super) const FIX_SAYS: &str = "Correct misheard words in a transcript transcribe made, \
each keeping when it is said, so captions timed to it stay in sync. Name each word by its \
number in the transcript as last shown and give what it says instead; an empty text drops a \
word heard where nothing was said. Every number is the one shown before this call. Answers \
with the transcript numbered afresh. The corrections belong to the library file, so every \
project using it reads them.";

/// `transcribe`'s arguments.
pub(super) fn transcribe_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "project": project_property(),
            "asset": {
                "type": "string",
                "description": "Id of the imported video or sound whose speech to hear."
            },
            "language": {
                "type": "string",
                "description": "The language spoken, as a code like `pt` or `en`. Omit it \
                                and it is detected; give it when the recording is short or \
                                mixes languages."
            },
            "again": {
                "type": "boolean",
                "description": "Hear it afresh even though a transcript is kept, replacing \
                                it and every correction made to it. Default false."
            }
        },
        "required": ["project", "asset"]
    })
}

/// `transcript_fix`'s arguments.
pub(super) fn fix_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "project": project_property(),
            "asset": {
                "type": "string",
                "description": "Id of the transcribed video or sound."
            },
            "fixes": {
                "type": "array",
                "description": "The corrections, each naming a word by its number in the \
                                transcript as last shown.",
                "items": {
                    "type": "object",
                    "properties": {
                        "word": {
                            "type": "integer",
                            "minimum": 1,
                            "description": "The word's number, from 1, as the transcript \
                                            was last shown."
                        },
                        "text": {
                            "type": "string",
                            "description": "What it says instead, punctuation included. \
                                            Empty drops the word."
                        }
                    },
                    "required": ["word", "text"]
                }
            }
        },
        "required": ["project", "asset", "fixes"]
    })
}

/// The recording `arguments` name in the caller's project, its library hash
/// and where its words sit in the project.
async fn recorded(
    caller: &Caller<'_>,
    arguments: &Value,
) -> Result<(AssetId, String, ProjectPath), String> {
    let project = project_id(arguments)?;
    let asset = arguments
        .get("asset")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|asset| !asset.is_empty())
        .ok_or("`asset` is required: the id of an imported video or sound")?;
    let asset = AssetId::new(asset);
    let stored = projects::open(&caller.toolbox.pool, caller.user, project)
        .await
        .map_err(|error| match error {
            ProjectError::NotFound => "there is no such project of yours".to_owned(),
            other => database(other),
        })?;
    let (found, path): (&Asset, &ProjectPath) =
        recording(&stored.document, &asset).map_err(|error| error.to_string())?;
    let sha256 = found
        .sha256
        .clone()
        .ok_or_else(|| format!("`{asset}` names no library file"))?;
    Ok((asset.clone(), sha256, Words::beside(path)))
}

/// A library failure, said to the caller.
fn said(error: LibraryError) -> String {
    match error {
        LibraryError::NotFound => "that file is not in your library".to_owned(),
        LibraryError::Invalid(why) => why,
        other => database(other),
    }
}

/// `transcribe`: the kept transcript, or a job queued to hear it.
pub(super) async fn transcribe(caller: &Caller<'_>, arguments: &Value) -> Result<Reply, String> {
    let language = arguments
        .get("language")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .map(str::parse::<Language>)
        .transpose()
        .map_err(|error| error.to_string())?;
    let again = arguments.get("again").and_then(Value::as_bool) == Some(true);
    let (asset, sha256, file) = recorded(caller, arguments).await?;
    let library = &caller.toolbox.library;
    let held = library
        .transcript(caller.user, &sha256)
        .await
        .map_err(said)?;
    if let (Some(words), false) = (held.words, again) {
        let kept = Transcribed {
            asset,
            file,
            words,
            heard: None,
        };
        return Ok(kept.said().into());
    }
    let payload = Payload {
        item: held.item,
        language: language.map(|code| code.as_str().to_owned()),
    };
    let job = library
        .transcribe(caller.user, &payload)
        .await
        .map_err(said)?;
    Ok(format!(
        "Transcribing `{asset}`: job {} ({}), free, about a minute for every half-minute of \
         speech. jobs with wait: true answers when it is done; then call transcribe again for \
         the words.",
        job.id, job.kind
    )
    .into())
}

/// `transcript_fix`: corrections kept on the library item.
pub(super) async fn fix(caller: &Caller<'_>, arguments: &Value) -> Result<Reply, String> {
    let fixes = arguments
        .get("fixes")
        .and_then(Value::as_array)
        .ok_or("`fixes` is required: a list of {word, text} corrections")?
        .iter()
        .map(|fix| {
            let word = fix.get("word").and_then(Value::as_u64);
            let text = fix.get("text").and_then(Value::as_str);
            match (word, text) {
                (Some(word), Some(text)) => Ok(Fix {
                    word: usize::try_from(word).unwrap_or(usize::MAX),
                    text: text.to_owned(),
                }),
                _ => Err("each fix is {word, text}: the word's number and what it says"),
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (asset, sha256, file) = recorded(caller, arguments).await?;
    let library = &caller.toolbox.library;
    let held = library
        .transcript(caller.user, &sha256)
        .await
        .map_err(said)?;
    let mut words = held
        .words
        .ok_or_else(|| format!("`{asset}` has no transcript to correct: transcribe it first"))?;
    let dropped = words.fix(&fixes).map_err(|error| error.to_string())?;
    library
        .keep_transcript(caller.user, held.item, &words)
        .await
        .map_err(said)?;
    let corrected = Corrected {
        transcript: Transcribed {
            asset,
            file,
            words,
            heard: None,
        },
        fixed: fixes.len(),
        dropped,
    };
    Ok(corrected.said().into())
}
