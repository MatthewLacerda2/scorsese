//! What a word is called where a page looks it up.
//!
//! A word is named by what it says, lowercased, with the punctuation around it
//! dropped: `Gradient,` is `gradient`, and `don't` keeps its apostrophe. The
//! second time a line says the same word it is `gradient@2`, the third
//! `gradient@3` — counted over the whole line, so trimming the clip never
//! renames a word that is still heard. A word with nothing left once its
//! punctuation is dropped (a lone dash) has no name.

use std::collections::HashMap;

use super::Words;

impl Words {
    /// Each word's name, in the order they are said; `None` for a word with
    /// nothing to call it by.
    pub fn names(&self) -> Vec<Option<String>> {
        let mut seen: HashMap<String, usize> = HashMap::new();
        self.words
            .iter()
            .map(|word| {
                let plain = plain(&word.text)?;
                let count = seen.entry(plain.clone()).or_default();
                *count += 1;
                Some(match *count {
                    1 => plain,
                    n => format!("{plain}@{n}"),
                })
            })
            .collect()
    }
}

/// `text` lowercased, without the punctuation either side of it.
fn plain(text: &str) -> Option<String> {
    let trimmed = text.trim_matches(|c: char| !c.is_alphanumeric());
    (!trimmed.is_empty()).then(|| trimmed.to_lowercase())
}
