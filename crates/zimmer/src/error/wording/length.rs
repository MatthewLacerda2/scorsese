//! What a song's length and level fields are refused with: `fit`, `fade`,
//! `tail` and `anchors`.
//!
//! Out of [`super`] because that file is at the size gate, and this is the
//! seam that group already had — the fields [`crate::song`] validates
//! together, because each is a property of the whole piece rather than of a
//! note in it. The parent's match hands every one of these variants here by
//! name, so a variant can only reach the catch-all arm below if it was added
//! to that list and not to this match — and then it prints its `Debug` form
//! rather than panicking inside a `Display`, which the first test to print it
//! finds.

use std::fmt;

use crate::error::SynthError;

impl SynthError {
    /// The words for a length or level refusal. Only ever called from
    /// [`SynthError::say`] with one of the seven variants it names.
    pub(super) fn say_length(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadFitSeconds { seconds, .. } => {
                write!(f, "song: `fit.seconds` must be positive, got {seconds}")
            }
            Self::FitLength { why, .. } => write!(f, "song: `fit` {why}"),
            Self::BadAnchor { section, why, .. } => {
                write!(f, "song: the anchor on section {section} {why}")
            }
            Self::BadFade { seconds, .. } => write!(
                f,
                "song: a fade must be zero or more seconds, got {seconds}"
            ),
            Self::StretchTooFar {
                bpm, needed, limit, ..
            } => write!(
                f,
                "song: fitting this at `stretch` needs {needed:.1} bpm against {bpm:.1} written, further than the {}% a piece survives — use `loop`, or change the arrangement",
                (limit * 100.0).round()
            ),
            Self::WrapWith { field, why, .. } => {
                write!(f, "song: `tail: wrap` cannot take {field}: {why}")
            }
            Self::WrapOverhang {
                overhang, length, ..
            } => write!(
                f,
                "song: `tail: wrap` would fold {overhang:.2} s of ring-out onto a {length:.2} s loop, and it would still be ringing the next time round — shorten the release or the effect tail, or lengthen the arrangement"
            ),
            // `say` hands over only the seven above, and a new variant cannot
            // reach here without being added to its list first. Should one
            // ever arrive regardless, it is still an error being described:
            // say what it is plainly rather than panic inside a `Display`.
            other => write!(f, "{other:?}"),
        }
    }
}
