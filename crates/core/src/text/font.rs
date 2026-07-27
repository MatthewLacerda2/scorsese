//! Which face a text asset is set in.
//!
//! Two names are reserved — `sans` and `serif`, the faces scorsese ships —
//! and anything else is read as a path to a font file inside the project.
//! Reserved names rather than a tagged object because the common case is one
//! word, and `{ "builtin": "serif" }` would be ceremony around a choice
//! between two things.

use serde::{Deserialize, Serialize};

use crate::path::ProjectPath;

/// The name reserved for the shipped sans face.
const SANS: &str = "sans";

/// The name reserved for the shipped serif face.
const SERIF: &str = "serif";

/// The face to set a text asset in.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum FontChoice {
    /// The shipped sans face — Liberation Sans, metric-compatible with Arial.
    /// The default: a caption reads as a caption in a sans face.
    #[default]
    Sans,
    /// The shipped serif face — Liberation Serif, metric-compatible with Times
    /// New Roman.
    Serif,
    /// A font file of the project's own, at a path relative to the project
    /// root like every other path in the document. This is what keeps the two
    /// shipped faces defaults rather than the whole vocabulary.
    File(
        /// Where the file sits inside the project, e.g. `assets/Inter.ttf`.
        ProjectPath,
    ),
}

impl FontChoice {
    /// The project-relative font file this names, if it names one at all.
    pub fn file(&self) -> Option<&ProjectPath> {
        match self {
            Self::File(path) => Some(path),
            _ => None,
        }
    }
}

impl From<String> for FontChoice {
    /// Infallible on purpose: a path that breaks the project-path rules is a
    /// [`crate::validate`] finding, reported alongside every other problem,
    /// rather than a parse failure that hides the rest of the document.
    fn from(text: String) -> Self {
        match text.as_str() {
            SANS => Self::Sans,
            SERIF => Self::Serif,
            _ => Self::File(ProjectPath::new(text)),
        }
    }
}

impl From<FontChoice> for String {
    fn from(font: FontChoice) -> Self {
        match font {
            FontChoice::Sans => SANS.to_owned(),
            FontChoice::Serif => SERIF.to_owned(),
            FontChoice::File(path) => path.as_str().to_owned(),
        }
    }
}
