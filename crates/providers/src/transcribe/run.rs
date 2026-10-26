//! Transcribing one asset of a project folder, and what that reads as.

use std::path::Path;

use scorsese_core::words::Words;
use scorsese_core::{AssetId, AssetKind, Project, ProjectPath};

use super::{Heard, Language, TranscribeError, Transcriber};

/// What a transcription is asked.
#[derive(Debug, Clone, Default)]
pub struct Asked {
    /// The language the speech is in; detected when `None`.
    pub language: Option<Language>,
    /// Hear it afresh even when a transcript is kept, replacing it and every
    /// correction made to it.
    pub again: bool,
}

/// A recording's transcript, where it is kept, and whether it was just heard.
#[derive(Debug, Clone, PartialEq)]
pub struct Transcribed {
    /// The asset.
    pub asset: AssetId,
    /// The file beside it.
    pub file: ProjectPath,
    /// The words, as kept.
    pub words: Words,
    /// What the server heard, when it was asked; `None` when the transcript
    /// already kept was read back.
    pub heard: Option<Heard>,
}

impl Transcribed {
    /// What every surface says: a line about it, then the words numbered
    /// ([`Words::listing`]).
    pub fn said(&self) -> String {
        let count = self.words.words.len();
        let words = if count == 1 { "word" } else { "words" };
        let head = match &self.heard {
            Some(heard) => format!(
                "`{}` transcribed ({}, {:.1} s): {count} {words}, kept in {}.",
                self.asset, heard.language, heard.duration, self.file
            ),
            None => format!(
                "`{}` was already transcribed: {count} {words} in {}, read back with any \
                 correction made to them. Ask again to hear it afresh, which replaces them.",
                self.asset, self.file
            ),
        };
        if count == 0 {
            return format!("{head} No speech was heard.");
        }
        format!("{head}\n{}", self.words.listing())
    }
}

/// Transcribes `asset`, an imported video or sound in the project at `root`,
/// by `transcriber`, its speech decoded by `decode` from the file — and writes
/// the words beside it ([`Words::beside`]). A transcript already there is read
/// back instead, unless [`Asked::again`].
pub fn transcribe(
    project: &Project,
    root: &Path,
    asset: &AssetId,
    asked: &Asked,
    transcriber: &dyn Transcriber,
    decode: impl FnOnce(&Path) -> Result<Vec<u8>, String>,
) -> Result<Transcribed, TranscribeError> {
    let found = project
        .asset(asset)
        .ok_or_else(|| TranscribeError::NoSuchAsset(asset.clone()))?;
    if found.kind == AssetKind::GeneratedAudio {
        return Err(TranscribeError::Generated(asset.clone()));
    }
    let path = found
        .path
        .as_ref()
        .filter(|_| Words::transcribable(found.kind))
        .ok_or_else(|| TranscribeError::NotARecording {
            asset: asset.clone(),
            kind: found.kind,
        })?;
    let source = path.resolve(root);
    if !source.is_file() {
        return Err(TranscribeError::Missing {
            asset: asset.clone(),
            path: path.clone(),
        });
    }
    let file = Words::beside(path);
    let done = |words, heard| Transcribed {
        asset: asset.clone(),
        file: file.clone(),
        words,
        heard,
    };
    if !asked.again
        && let Some(kept) = Words::of(found, root)
    {
        return Ok(done(kept, None));
    }
    let wav = decode(&source).map_err(|why| TranscribeError::Decode {
        asset: asset.clone(),
        why,
    })?;
    let heard = transcriber.transcribe(wav, asked.language.as_ref())?;
    write(root, &file, &heard.words)?;
    Ok(done(heard.words.clone(), Some(heard)))
}

/// Writes `words` at `file`, by way of a file beside it, so a transcript is
/// never read half-written.
fn write(root: &Path, file: &ProjectPath, words: &Words) -> Result<(), TranscribeError> {
    let target = file.resolve(root);
    let writing = target.with_extension("json.writing");
    let failed = |source| TranscribeError::Write {
        path: file.clone(),
        source,
    };
    std::fs::write(&writing, words.to_json()).map_err(failed)?;
    std::fs::rename(&writing, &target).map_err(failed)
}
