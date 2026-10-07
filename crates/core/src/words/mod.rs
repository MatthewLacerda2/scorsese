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
//! guessed.** Imported audio has none, and neither does a narration generated
//! before timings were asked for. Getting them would mean paying for the line
//! again, which nothing does on its own: only a new generation brings them.
//!
//! [`Words::placed`] carries the words onto the timeline through the clip that
//! plays the line — its `start`, `source_in` and `speed` — and names each one
//! the way a page addresses it ([`Words::names`]); [`saying`] says which word
//! each narration is on at an instant.

mod at;
mod name;
mod placed;

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{Asset, AssetKind, GenerationState, ProjectPath};

pub use at::{Saying, saying};
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

    /// The timings of `asset`'s line, if it is a generated narration whose
    /// audio has them beside it. `None` for every other asset, for a line
    /// generated before timings were kept, and for a file that will not read.
    pub fn of(asset: &Asset, project_root: &Path) -> Option<Self> {
        if asset.kind != AssetKind::GeneratedAudio
            || asset.state != Some(GenerationState::Generated)
        {
            return None;
        }
        let file = Self::beside(asset.path.as_ref()?).resolve(project_root);
        let text = std::fs::read_to_string(file).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// The file's contents, as [`Words::of`] reads them back.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("words always serialise")
    }
}

#[cfg(test)]
mod tests;
