//! How a text asset comes into view a piece at a time.
//!
//! **The block says how, a keyframe track says when.** The `reveal` property —
//! a number from `0` (nothing shown) to `1` (all of it) — is animated on the
//! clip like any other, with the same keyframes and the same easings. This
//! block is the part that is not a number over time: which pieces the text is
//! cut into, how far each one rises as it arrives, and how much the pieces
//! overlap. So a word-by-word caption is a `reveal` track going `0 → 1` and,
//! if the defaults suit, nothing else at all.
//!
//! The pieces are counted in reading order across the whole block — every
//! line, top to bottom, each left to right — and `reveal` sweeps them evenly:
//! at `0.5`, half of the pieces are in or on their way in. A cluster the eye
//! reads as one character is one piece, which is what keeps an emoji with a
//! skin tone, or a flag, from arriving in halves.

use serde::{Deserialize, Serialize};

/// What a text is cut into when it is revealed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RevealUnit {
    /// One character at a time — the typewriter. An emoji, a flag or a letter
    /// with its accent is one character here, the way it is on screen.
    Char,
    /// One word at a time — the caption that assembles itself with the voice.
    /// The default, because it is the one people mean by "animate the text".
    #[default]
    Word,
    /// One line at a time — the list that builds.
    Line,
}

/// How a text asset's pieces arrive when its `reveal` property animates.
///
/// Every field has a default and an absent block means all of them, so a
/// `reveal` keyframe track on a text clip with no block at all reveals word by
/// word, rising a little, overlapping by half.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Reveal {
    /// Which pieces the text arrives in.
    pub unit: RevealUnit,
    /// How far below its place each piece starts, as a fraction of the text's
    /// own **size** — so the same number is the same lift for a caption and a
    /// title. `0` fades in place; negative drops in from above.
    ///
    /// Of the size rather than the frame, because it is a distance *between
    /// letters and their line*, and that scales with the letters.
    pub rise: f64,
    /// How far one piece gets through its own entrance before the next one
    /// starts, from `0` to `1`. `1` is strictly one after another — a
    /// typewriter — and `0` is every piece at once, which is a plain fade.
    ///
    /// **The easing belongs to each piece.** The `reveal` track's keyframe
    /// easing shapes every piece's own entrance, not the sweep across them: the
    /// sweep is even, and `back_out` makes each word overshoot its line and
    /// settle, which is the pop a title preset has. A piece's opacity never
    /// goes past solid or below nothing; its rise overshoots freely.
    pub stagger: f64,
}

impl Reveal {
    /// A fifth of the text's size: enough to read as movement, little enough
    /// that a word never looks like it came from the line below.
    pub const DEFAULT_RISE: f64 = 0.2;

    /// Each piece halfway in before the next starts — a cascade rather than a
    /// queue.
    pub const DEFAULT_STAGGER: f64 = 0.5;
}

impl Default for Reveal {
    fn default() -> Self {
        Self {
            unit: RevealUnit::default(),
            rise: Self::DEFAULT_RISE,
            stagger: Self::DEFAULT_STAGGER,
        }
    }
}
