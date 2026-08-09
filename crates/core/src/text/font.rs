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

/// The lightest weight OpenType allows on the `wght` axis.
///
/// The bounds are the *format's*, not any font's. A file declares its own,
/// usually narrower range — Manrope stops at 200 and 800 — and only the file
/// can say what that is, so checking it belongs where the file is opened. What
/// is checkable from the document alone is that the number is a weight at all,
/// and `0` or `70000` is not one whatever face it is aimed at.
pub const MIN_WEIGHT: u16 = 1;

/// The heaviest weight OpenType allows on the `wght` axis.
pub const MAX_WEIGHT: u16 = 1000;

/// The face to set a text asset in.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum FontChoice {
    /// The shipped sans face — Inter, as one variable file covering `wght` 100
    /// to 900. The default: a caption reads as a caption in a sans face.
    #[default]
    Sans,
    /// The shipped serif face — Source Serif 4, variable over `wght` 200 to
    /// 900.
    Serif,
    /// A font file of the project's own, at a path relative to the project
    /// root like every other path in the document. This is what keeps the two
    /// shipped faces defaults rather than the whole vocabulary.
    File(
        /// Where the file sits inside the project, e.g.
        /// `assets/Manrope[wght].ttf`.
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
    /// [`crate::Project::validate`] finding, reported alongside every other problem,
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
