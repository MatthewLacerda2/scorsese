//! What a template is *for*, in a person's words (#560).
//!
//! A template already carries its assets' and tracks' notes, but those are
//! about one thing each; nothing said what the whole piece is — "the intro I
//! open every daily video with", "the lower third for guests". So a template
//! has a description of its own, and this is the one rule for what one may be,
//! whoever keeps templates: the hosted server today, a local folder the day it
//! has templates.
//!
//! **Beside the template's document, never inside it.** A template is a
//! `project.json` document ([`super`]), and `project.json` refuses unknown
//! fields, so a description *in* it would be a format change — a schema bump
//! and a migration — for a field no project has. Whoever stores a template
//! stores its description next to it, the way the server keeps a column beside
//! the document.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The longest a description may be, in characters: a paragraph, not a script.
pub const MAX_DESCRIPTION_CHARS: usize = 2000;

/// A template's description: trimmed, never empty, at most
/// [`MAX_DESCRIPTION_CHARS`] long.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Description(String);

/// Why a description was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "a template's description is at most {MAX_DESCRIPTION_CHARS} characters; this one is \
     {chars} — say what the template is for, not everything in it"
)]
pub struct DescriptionTooLong {
    /// How long the one given was.
    pub chars: usize,
}

impl Description {
    /// `text`, trimmed — `None` when nothing is left, since an empty
    /// description says nothing and is the same as none.
    pub fn new(text: &str) -> Result<Option<Self>, DescriptionTooLong> {
        let text = text.trim();
        let chars = text.chars().count();
        if chars > MAX_DESCRIPTION_CHARS {
            return Err(DescriptionTooLong { chars });
        }
        Ok((!text.is_empty()).then(|| Self(text.to_owned())))
    }

    /// The words.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Description {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        Self::new(&text)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "a template's description cannot be empty".to_owned())
    }
}

impl From<Description> for String {
    fn from(description: Description) -> Self {
        description.0
    }
}

impl fmt::Display for Description {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_description_is_trimmed_and_blank_is_none() {
        let said = Description::new("  the intro for daily videos \n").unwrap();
        assert_eq!(
            said.map(String::from).as_deref(),
            Some("the intro for daily videos")
        );
        assert_eq!(Description::new(" \n\t ").unwrap(), None);
    }

    #[test]
    fn the_cap_counts_characters_not_bytes() {
        let longest = "é".repeat(MAX_DESCRIPTION_CHARS);
        assert!(Description::new(&longest).unwrap().is_some());
        let over = format!("{longest}e");
        assert_eq!(
            Description::new(&over),
            Err(DescriptionTooLong {
                chars: MAX_DESCRIPTION_CHARS + 1
            })
        );
    }

    #[test]
    fn it_reads_and_writes_as_a_plain_string_and_refuses_an_empty_one() {
        let said: Description = serde_json::from_str("\" an outro \"").unwrap();
        assert_eq!(said.as_str(), "an outro");
        assert_eq!(serde_json::to_string(&said).unwrap(), "\"an outro\"");
        assert!(serde_json::from_str::<Description>("\"  \"").is_err());
        assert_eq!(said.to_string(), "an outro");
    }
}
