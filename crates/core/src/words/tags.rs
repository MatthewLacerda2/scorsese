//! Audio tags are directions, not words (#1061).
//!
//! On the `expressive` model a prompt's square brackets direct the reading —
//! `[gruff, not looking up] We're closed.` — and are not said. The vendor
//! still times their characters, so a line folded as written would start with
//! `[gruff,` and `up]` as words, and every reader of the timings (captions,
//! pages, `project_describe`) would take them for something spoken. On the
//! other models a bracket is read aloud like any other character, so the rule
//! is the model's: [`SpeechModel::takes_tags`].

use crate::SpeechModel;

use super::{Word, Words};

impl Words {
    /// Folds a reading's per-character alignment into words, as
    /// [`Words::from_characters`] does, leaving out the audio tags `model`
    /// takes as directions. Dropped before folding, so a word written against
    /// its tag (`[sighs]Okay`) still starts where it is said.
    pub fn from_reading(
        characters: impl IntoIterator<Item = (char, f64, f64)>,
        model: SpeechModel,
    ) -> Self {
        let characters: Vec<_> = characters.into_iter().collect();
        if !model.takes_tags() {
            return Self::from_characters(characters);
        }
        Self::from_characters(untagged(characters, |&(character, ..)| character))
    }

    /// The words as `model` spoke them: without its audio tags, for a line
    /// folded before tags were dropped (#1061). Only whole words are timed by
    /// then, so a word that shared one with a tag keeps the tag's start.
    pub fn spoken_by(self, model: SpeechModel) -> Self {
        if !model.takes_tags() {
            return self;
        }
        let characters = self
            .words
            .iter()
            .enumerate()
            .flat_map(|(at, word)| word.text.chars().map(move |character| (at, character)));
        let mut said = vec![String::new(); self.words.len()];
        for (at, character) in untagged(characters.collect(), |&(_, character)| character) {
            said[at].push(character);
        }
        let words = self
            .words
            .into_iter()
            .zip(said)
            .filter(|(_, text)| !text.is_empty())
            .map(|(word, text)| Word { text, ..word })
            .collect();
        Self { words }
    }
}

/// `items` without every span from a `[` to the next `]`, both included. A
/// `[` that never closes is no tag, and stays.
fn untagged<T>(items: Vec<T>, character: impl Fn(&T) -> char) -> Vec<T> {
    let characters: Vec<char> = items.iter().map(character).collect();
    let mut dropped = vec![false; characters.len()];
    let mut at = 0;
    while at < characters.len() {
        match characters[at..].iter().position(|&c| c == ']') {
            Some(close) if characters[at] == '[' => {
                dropped[at..=at + close].fill(true);
                at += close + 1;
            }
            _ => at += 1,
        }
    }
    items
        .into_iter()
        .zip(dropped)
        .filter(|(_, dropped)| !dropped)
        .map(|(item, _)| item)
        .collect()
}
