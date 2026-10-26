//! A transcript read as a numbered list, and a misheard word corrected in it
//! (#1549).
//!
//! Whisper mishears a name or a word here and there. A correction changes
//! what a word says and never when: the timings are what the recording
//! measured, and a caption or a page timed to them stays in sync. A word
//! Whisper heard where nothing was said is dropped by correcting it to
//! nothing.

use super::Words;

/// One correction: the word numbered `word`, as [`Words::listing`] numbers
/// them from 1, now reads `text`. Nothing left once trimmed drops the word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    /// Which word, counting from 1.
    pub word: usize,
    /// What it says instead.
    pub text: String,
}

/// Why a set of corrections changed nothing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FixError {
    /// A correction names a word the transcript does not have.
    #[error("there is no word {word}: the transcript has {count}, numbered from 1")]
    NoSuchWord {
        /// The number given.
        word: usize,
        /// How many words there are.
        count: usize,
    },
    /// Two corrections name one word.
    #[error("word {0} is corrected twice: give it once")]
    Twice(usize),
}

impl Words {
    /// One line a word: its number, when it starts and ends in seconds, and
    /// what it says.
    pub fn listing(&self) -> String {
        self.words
            .iter()
            .enumerate()
            .map(|(i, word)| {
                format!(
                    "{:>4}  {:.2}–{:.2}  {}",
                    i + 1,
                    word.start,
                    word.end,
                    word.text
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Applies every correction, each word keeping its times, and returns how
    /// many words were dropped. Every number is the listing's before the
    /// fix, so dropping one renumbers none of the others. Nothing changes when
    /// one of them is refused.
    pub fn fix(&mut self, fixes: &[Fix]) -> Result<usize, FixError> {
        let count = self.words.len();
        let mut seen = vec![false; count];
        for fix in fixes {
            if fix.word == 0 || fix.word > count {
                return Err(FixError::NoSuchWord {
                    word: fix.word,
                    count,
                });
            }
            if std::mem::replace(&mut seen[fix.word - 1], true) {
                return Err(FixError::Twice(fix.word));
            }
        }
        for fix in fixes {
            fix.text
                .trim()
                .clone_into(&mut self.words[fix.word - 1].text);
        }
        self.words.retain(|word| !word.text.is_empty());
        Ok(count - self.words.len())
    }
}
