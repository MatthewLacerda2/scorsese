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
    /// What colour a `color` asset is.
    ///
    /// The other inline kind, and the whole of what it carries — a `color`
    /// asset has no content, only appearance. Required rather than defaulted:
    /// a background is the largest thing on screen, and one that came out
    /// white because nobody chose is a shot rendered wrong in a way no error
    /// ever mentioned.
    Color,
    /// The outline a `shape` asset draws, and the two colours it is drawn in.
    ///
    /// The third inline kind's whole content, required by it and refused
    /// everywhere else on the reasoning every stray field is refused on: a
    /// rectangle described on a video asset would be drawn by nothing, and
    /// silence about it would look exactly like having drawn it.
    Shape,
    /// The rest of a generated video's brief: tier, raster, length, aspect and
    /// the stills it is built from.
    ///
    /// One field for all of them because they are one request, and refused
    /// anywhere but `generated_video` for the reason every stray field is: a
    /// resolution on a narration prompt would be read by nothing, and silence
    /// about it would look exactly like having honoured it.
    Video,
    /// The rest of a spoken line's brief: model, voice, language, seed.
    ///
    /// The sibling of [`AssetField::Video`] and refused everywhere but
    /// `generated_audio`, on the same reasoning read the other way: a voice on
    /// a Veo shot would be handed to nobody, and nothing would say so.
    Speech,
    /// The provider's name for work in flight.
    ///
    /// Only the kinds that queue with somebody else have one. A ticket on an
    /// imported file names work nobody is doing.
    Operation,
    /// When a provider took the request.
    ///
    /// Only the generated kinds queue, so only they can have been handed over.
    QueuedAt,
    /// What realising the asset cost, in US cents.
    ///
    /// Only the prompted kinds can have cost anything. A synthesised asset is
    /// arithmetic and an imported one is a file that was already there, so a
    /// price on either is a number that came from nowhere.
    Cost,
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
            Self::Color => "color",
            Self::Shape => "shape",
            Self::Video => "video",
            Self::Speech => "speech",
            Self::Operation => "operation",
            Self::QueuedAt => "queued_at",
            Self::Cost => "estimated_cost_cents",
        }
    }
}

impl fmt::Display for AssetField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
