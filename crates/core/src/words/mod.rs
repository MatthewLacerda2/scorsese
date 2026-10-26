//! When each word of a spoken line is said (#811).
//!
//! A narration generated with timings keeps them in a file of its own beside
//! the audio — `generated/<asset>-<brief hash>.words.json` next to
//! `generated/<asset>-<brief hash>.mp3` — so it is content-addressed exactly
//! as the audio is: a cache hit on an unchanged brief finds both, and nothing
//! in `project.json` changes. What the file holds is [`Words`]: each word as
//! it was written in the line, with when it starts and ends **in seconds of
//! the audio file**.
//!
//! **A line with no such file has no word timings, and that is said, never
//! guessed.** A narration generated before timings were asked for has none.
//! Getting them would mean paying for the line again, which nothing does on
//! its own: only a new generation brings them.
//!
//! **Imported speech gets them by being transcribed** (#1549): a local
//! Whisper hears an imported video or sound and its words are written in the
//! same file, beside the asset — `assets/sketch.words.json` next to
//! `assets/sketch.mp4` — so captions, pages and `project_describe` read them
//! as they read a narration's. The agent's corrections ([`Words::fix`]) make
//! that file work nothing can rebuild, so it is never under `cache/`.
//!
//! [`Words::placed`] carries the words onto the timeline through the clip that
//! plays the line — its `start`, `source_in` and `speed` — and names each one
//! the way a page addresses it ([`Words::names`]); [`saying`] says which word
//! each narration is on at an instant.

mod at;
mod fix;
mod name;
mod placed;
mod tags;

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{Asset, AssetKind, GenerationState, ProjectPath};

pub use at::{Saying, saying};
pub use fix::{Fix, FixError};
pub use placed::Placed;

/// One word of a line, as written, and when it is said.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Word {
    /// The word with the punctuation it was written with — `back.`, not
    /// `back` — so a caption made from it reads as the line did.
    pub text: String,
    /// When it begins, in seconds of the audio file.
    pub start: f64,
    /// When it ends, in the same seconds.
    pub end: f64,
}

/// Every word of one spoken line, in the order they are said.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Words {
    /// The words.
    pub words: Vec<Word>,
}

impl Words {
    /// Folds a per-character alignment into words: whitespace separates them,
    /// and a word runs from its first character's start to its last one's end.
    pub fn from_characters(characters: impl IntoIterator<Item = (char, f64, f64)>) -> Self {
        let mut words = Vec::new();
        let mut current: Option<Word> = None;
        for (character, start, end) in characters {
            if character.is_whitespace() {
                words.extend(current.take());
                continue;
            }
            let word = current.get_or_insert_with(|| Word {
                text: String::new(),
                start,
                end,
            });
            word.text.push(character);
            word.end = end;
        }
        words.extend(current);
        Self { words }
    }

    /// Where the timings of the audio at `audio` are kept: beside it, under
    /// the same name with `.words.json` for its extension.
    pub fn beside(audio: &ProjectPath) -> ProjectPath {
        let path = audio.as_str();
        let stem = match path.rfind('.') {
            Some(dot) if !path[dot..].contains('/') => &path[..dot],
            _ => path,
        };
        ProjectPath::new(format!("{stem}.words.json"))
    }

    /// The timings of what `asset` says, when they are beside it: a generated
    /// narration's, or an imported recording's transcript
    /// ([`Words::transcribable`]). `None` for every other asset, for a line
    /// generated before timings were kept, for a recording nobody transcribed,
    /// and for a file that will not read. An `expressive` line's audio tags
    /// are left out even when its file still holds them, as one folded before
    /// #1061 does ([`Words::spoken_by`]).
    pub fn of(asset: &Asset, project_root: &Path) -> Option<Self> {
        let file = Self::beside(asset.path.as_ref()?).resolve(project_root);
        let read = || -> Option<Self> {
            let text = std::fs::read_to_string(&file).ok()?;
            serde_json::from_str(&text).ok()
        };
        if Self::transcribable(asset.kind) {
            return read();
        }
        if asset.kind != AssetKind::GeneratedAudio
            || asset.state != Some(GenerationState::Generated)
        {
            return None;
        }
        Some(read()?.spoken_by(asset.speech_request().model))
    }

    /// Whether an asset of `kind` is a recording a transcript can be made of:
    /// an imported video or sound. A generated line has its timings from the
    /// provider that spoke it.
    pub fn transcribable(kind: AssetKind) -> bool {
        matches!(kind, AssetKind::Video | AssetKind::Audio)
    }

    /// The file's contents, as [`Words::of`] reads them back.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("words always serialise")
    }
}

#[cfg(test)]
mod tests;
