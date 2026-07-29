//! Which field of an asset a problem is about.
//!
//! Two of validation's findings are the same sentence with a different word in
//! it — *this kind needs that field*, and *this kind has no use for it* — so
//! they are one variant each, carrying the word. Before this existed there was
//! a variant per field per direction, which meant the reasoning for a field was
//! written twice and adding a field meant adding two errors.
//!
//! So this is where the reasoning lives now: one place per field saying why it
//! belongs where it does, rather than the same explanation split across a
//! "missing" error and a "stray" one.

use std::fmt;

/// A field an asset may carry, named the way `project.json` spells it.
///
/// Only the fields whose presence depends on `kind`. `id` and `kind` are
/// always required and `sha256` and `media` are always optional, so neither
/// can be missing from a kind that wants it or stray on one that does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetField {
    /// Where the media is, relative to the project root.
    ///
    /// Required by every file-backed kind, which is every kind but the inline
    /// ones: without it there is nothing on disk to decode.
    Path,
    /// A text asset's content, carried inline.
    ///
    /// One of the two kinds with no file behind it, so this *is* the asset —
    /// there is no other place its words could come from.
    Text,
    /// How a text asset's content looks: font, size, colour, alignment.
    ///
    /// Optional even on `text`, where an absent style means every default. On
    /// anything else it would be read by nothing, so it is refused rather than
    /// ignored: a style on a video asset is a `kind` that was meant to be
    /// `text`.
    Style,
    /// The sentence a provider generates from.
    ///
    /// Required by the prompt-backed kinds and refused elsewhere — including
    /// on `synth_audio`, whose brief is a `recipe`. A prompt beside a recipe
    /// is two briefs for one asset, and nothing could say which was meant.
    Prompt,
    /// The document synthesis reads, by convention under `recipes/`.
    ///
    /// The other kind of brief: local, free and deterministic, where a prompt
    /// costs money and needs a network. Required by the synthesised kinds and
    /// refused everywhere else.
    Recipe,
    /// Where a generated asset sits in the sketch lifecycle.
    ///
    /// Required by every generated kind, because without it GO cannot tell
    /// whether this has been paid for. Refused on the rest: an imported file
    /// is simply there, and has no lifecycle to be at a point in.
    State,
}

impl AssetField {
    /// The field's name as `project.json` writes it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Text => "text",
            Self::Style => "style",
            Self::Prompt => "prompt",
            Self::Recipe => "recipe",
            Self::State => "state",
        }
    }
}

impl fmt::Display for AssetField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
