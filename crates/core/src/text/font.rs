//! Which face a text asset is set in.
//!
//! **A bare word is a face scorsese ships; anything with a slash or a dot in it
//! is a font file inside the project.** That is a rule about the *shape* of the
//! string, and shape is what this crate is allowed to know.
//!
//! Which words are real faces is `scorsese-compositor`'s to say, next to the
//! `include_bytes!` that make them real — the same arrangement the animatable
//! properties already have, and for the same reason. Core says *a font is a
//! name or a file*, which is a property **type**; the moment it held the list
//! of names it would hold property **values**, and adding a face would become a
//! change to the model.
//!
//! Two names remain in this file and neither is a catalogue. [`DEFAULT_FONT`]
//! is the face a `style` that says nothing gets, which core has to pick for the
//! same reason it picks white for the colour. `MIN_WEIGHT` and `MAX_WEIGHT` are
//! OpenType's own bounds, which are a fact about the format rather than about
//! any font.

use serde::{Deserialize, Serialize};

use crate::path::ProjectPath;

/// The face a `style` that names none is set in.
///
/// A name rather than a file, so the default survives the shipped set changing
/// under it — which it has, once already.
pub const DEFAULT_FONT: &str = "sans";

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum FontChoice {
    /// A face scorsese ships, by name — `sans`, `serif`, `liberation-sans`.
    ///
    /// Whether the name is one that exists is **not** decided here. It is a
    /// question about which files this build compiled in, so it is answered
    /// where those files are, and reported by `scorsese check` and by the
    /// render rather than by the parser.
    Named(String),
    /// A font file of the project's own, at a path relative to the project
    /// root like every other path in the document. This is what keeps the
    /// shipped faces a starting point rather than the whole vocabulary.
    File(
        /// Where the file sits inside the project, e.g.
        /// `assets/Manrope[wght].ttf`.
        ProjectPath,
    ),
}

impl Default for FontChoice {
    fn default() -> Self {
        Self::Named(DEFAULT_FONT.to_owned())
    }
}

impl FontChoice {
    /// The project-relative font file this names, if it names one at all.
    pub fn file(&self) -> Option<&ProjectPath> {
        match self {
            Self::File(path) => Some(path),
            Self::Named(_) => None,
        }
    }

    /// The shipped face this names, if it names one at all.
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Named(name) => Some(name),
            Self::File(_) => None,
        }
    }
}

/// Whether a `font` string is meant as a path rather than as a face's name.
///
/// A slash or a dot. Every path a project can legally write has at least one of
/// them — a font file has an extension, and one in `assets/` has a separator —
/// and no name of a shipped face has either, because names are bare words.
///
/// The important half is what this does with a word nobody ships. `"Arial"` is
/// read as a **name**, so the answer is "there is no face called Arial, here are
/// the ones there are", rather than the path error that reading it as a
/// filename would produce. The wrong thing said clearly is worse than useless
/// when the right thing is one sentence away.
fn looks_like_a_path(text: &str) -> bool {
    text.contains('/') || text.contains('.')
}

impl From<String> for FontChoice {
    /// Infallible on purpose: a path that breaks the project-path rules is a
    /// [`crate::Project::validate`] finding, reported alongside every other problem,
    /// rather than a parse failure that hides the rest of the document.
    fn from(text: String) -> Self {
        if looks_like_a_path(&text) {
            Self::File(ProjectPath::new(text))
        } else {
            Self::Named(text)
        }
    }
}

impl From<FontChoice> for String {
    fn from(font: FontChoice) -> Self {
        match font {
            FontChoice::Named(name) => name,
            FontChoice::File(path) => path.as_str().to_owned(),
        }
    }
}
