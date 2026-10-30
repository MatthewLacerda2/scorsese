//! Transcribing one asset of a project folder, and what that reads as.

use std::path::Path;

use scorsese_core::words::{Fix, Words};
use scorsese_core::{Asset, AssetId, AssetKind, Project, ProjectPath};

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
    /// The recording's hash when it was heard or read, which [`keep`] checks
    /// the project still has.
    pub sha256: Option<String>,
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

/// Why a recording's speech did not come out of its file, as the `decode`
/// a caller hands [`transcribe`] says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Undecoded {
    /// Whoever asked stopped it while it was decoded.
    Stopped,
    /// What the decoder said.
    Failed(String),
}

/// What [`transcribe`] and [`hear`] are handed besides the asset.
pub struct Hearing<'a, D> {
    /// What is asked.
    pub asked: &'a Asked,
    /// Who hears it.
    pub transcriber: &'a dyn Transcriber,
    /// The recording's speech, out of its file: a WAV, at most
    /// [`LONGEST`](super::LONGEST) of it.
    pub decode: D,
    /// Whether whoever asked has stopped it.
    pub stop: &'a dyn Fn() -> bool,
}

/// Transcribes `asset`, an imported video or sound in the project at `root`
/// ([`hear`]), and writes the words beside it ([`keep`]) when they were just
/// heard.
pub fn transcribe(
    project: &Project,
    root: &Path,
    asset: &AssetId,
    hearing: Hearing<'_, impl FnOnce(&Path) -> Result<Vec<u8>, Undecoded>>,
) -> Result<Transcribed, TranscribeError> {
    let done = hear(project, root, asset, hearing)?;
    keep(project, root, &done)?;
    Ok(done)
}

/// Hears `asset`, an imported video or sound in the project at `root`, by its
/// [`Hearing`] — or reads back the transcript already beside it
/// ([`Words::transcript`]) unless [`Asked::again`]. Writes nothing: what was
/// heard is [`keep`]'s, so a caller that holds the project while it writes
/// need not hold it while the server listens.
pub fn hear(
    project: &Project,
    root: &Path,
    asset: &AssetId,
    hearing: Hearing<'_, impl FnOnce(&Path) -> Result<Vec<u8>, Undecoded>>,
) -> Result<Transcribed, TranscribeError> {
    let (found, path) = recording(project, asset)?;
    let source = path.resolve(root).ok();
    let Some(source) = source.filter(|source| source.is_file()) else {
        return Err(TranscribeError::Missing {
            asset: asset.clone(),
            path: path.clone(),
        });
    };
    let file = Words::transcript(path);
    let done = |words, heard| Transcribed {
        asset: asset.clone(),
        file: file.clone(),
        words,
        sha256: found.sha256.clone(),
        heard,
    };
    if !hearing.asked.again
        && let Some(kept) = Words::of(found, root)
    {
        return Ok(done(kept, None));
    }
    if let Some(seconds) = found
        .media
        .as_ref()
        .and_then(|media| media.duration_seconds)
        .filter(|&seconds| seconds > super::LONGEST.as_secs_f64())
    {
        return Err(TranscribeError::TooLong {
            asset: asset.clone(),
            seconds,
        });
    }
    let wav = (hearing.decode)(&source).map_err(|why| match why {
        Undecoded::Stopped => TranscribeError::Stopped,
        Undecoded::Failed(why) => TranscribeError::Decode {
            asset: asset.clone(),
            why,
        },
    })?;
    let heard =
        hearing
            .transcriber
            .transcribe(wav, hearing.asked.language.as_ref(), hearing.stop)?;
    Ok(done(heard.words.clone(), Some(heard)))
}

/// Writes what [`hear`] just heard beside its recording, when `project` —
/// read again as it is now — still has that recording, at the same file with
/// the same hash; otherwise [`TranscribeError::Changed`], and nothing is
/// written for a recording removed or replaced while it was heard. A
/// transcript only read back is already there, and is left alone.
pub fn keep(project: &Project, root: &Path, done: &Transcribed) -> Result<(), TranscribeError> {
    if done.heard.is_none() {
        return Ok(());
    }
    let changed = || TranscribeError::Changed(done.asset.clone());
    let (found, path) = recording(project, &done.asset).map_err(|_| changed())?;
    if Words::transcript(path) != done.file || found.sha256 != done.sha256 {
        return Err(changed());
    }
    write(root, &done.file, &done.words)
}

/// Writes `words` at `file`, by way of a file beside it, so a transcript is
/// never read half-written.
fn write(root: &Path, file: &ProjectPath, words: &Words) -> Result<(), TranscribeError> {
    let failed = |source| TranscribeError::Write {
        path: file.clone(),
        source,
    };
    let target = file
        .resolve(root)
        .map_err(|refused| failed(refused.into()))?;
    let writing = target.with_extension("json.writing");
    std::fs::write(&writing, words.to_json()).map_err(failed)?;
    std::fs::rename(&writing, &target).map_err(failed)
}

/// A transcript after a correction, and how much it changed.
#[derive(Debug, Clone, PartialEq)]
pub struct Corrected {
    /// The transcript as now kept.
    pub transcript: Transcribed,
    /// How many corrections were made.
    pub fixed: usize,
    /// How many of them dropped a word.
    pub dropped: usize,
}

impl Corrected {
    /// What every surface says: a line, then the words numbered afresh.
    pub fn said(&self) -> String {
        let plural =
            |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
        format!(
            "`{}` corrected in {}: {}, {} dropped, each kept word at its own times. \
             Numbered afresh:\n{}",
            self.transcript.asset,
            self.transcript.file,
            plural(self.fixed, "fix", "fixes"),
            plural(self.dropped, "word", "words"),
            self.transcript.words.listing()
        )
    }
}

/// Corrects `asset`'s transcript ([`Words::fix`]) and writes it back, each
/// word keeping its times.
pub fn correct(
    project: &Project,
    root: &Path,
    asset: &AssetId,
    fixes: &[Fix],
) -> Result<Corrected, TranscribeError> {
    let (found, path) = recording(project, asset)?;
    let mut words =
        Words::of(found, root).ok_or_else(|| TranscribeError::Untranscribed(asset.clone()))?;
    let dropped = words.fix(fixes)?;
    let file = Words::transcript(path);
    write(root, &file, &words)?;
    Ok(Corrected {
        transcript: Transcribed {
            asset: asset.clone(),
            file,
            words,
            sha256: found.sha256.clone(),
            heard: None,
        },
        fixed: fixes.len(),
        dropped,
    })
}

/// `asset`, when it is a recording a transcript can be made of, and its file:
/// what [`transcribe`] and [`correct`] refuse, for a caller that keeps the
/// words elsewhere — the web app, on the library's row.
pub fn recording<'a>(
    project: &'a Project,
    asset: &AssetId,
) -> Result<(&'a Asset, &'a ProjectPath), TranscribeError> {
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
    Ok((found, path))
}
