//! How much of a shape's line is drawn, and how it is broken into dashes.
//!
//! Two types, because the same three numbers live in two units. [`Trace`] is
//! what one instant of a clip resolves to, in the document's own units — it
//! sits in [`crate::Properties`] beside `opacity`, resolved from the same
//! keyframe tracks. [`Stroking`] is the same thing in pixels, with the
//! document's static `dash` pattern beside it, which is what the drawing takes.
//! Turning one into the other needs the raster, so it happens where the raster
//! is known, exactly as a shape's size does.
//!
//! **The stroke only.** A filled box whose border is trimmed keeps its whole
//! fill: a trim says how much of the *line* has been drawn, and a fill has no
//! line to be part way along.

use scorsese_core::KeyframeTrack;

use crate::properties::path;

/// The animated half of a shape's line at one instant, in document units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trace {
    /// Where the drawn part of the line starts, as a fraction of the outline's
    /// length: `0.0` its start.
    pub trim_start: f64,
    /// Where the drawn part ends: `1.0` all the way.
    pub trim_end: f64,
    /// How far the dash pattern has moved along the line, toward its end, as
    /// a fraction of the raster's height. Nothing without a `dash`.
    pub dash_offset: f64,
}

impl Trace {
    /// The whole line, and dashes where the pattern puts them — what a shape
    /// with none of these keyframed looks like.
    pub const WHOLE: Self = Self {
        trim_start: 0.0,
        trim_end: 1.0,
        dash_offset: 0.0,
    };

    /// Whether any of `tracks` animates one of these — the question that
    /// decides whether a shape can be drawn once for a whole segment or has to
    /// be drawn again for every frame.
    pub fn is_animated_by(tracks: &[KeyframeTrack]) -> bool {
        tracks.iter().any(|track| {
            matches!(
                track.property.as_str(),
                path::TRIM_START | path::TRIM_END | path::DASH_OFFSET
            )
        })
    }
}

impl Default for Trace {
    fn default() -> Self {
        Self::WHOLE
    }
}

/// How a shape's line is drawn at one instant, in pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroking {
    /// Where the drawn part starts, as a fraction of the outline's length.
    /// Clamped to `0.0..=1.0` when drawn, never refused: an easing that
    /// overshoots is an ordinary animation and must not fail a render.
    pub trim_start: f32,
    /// Where it ends, clamped the same way. At or before `trim_start` nothing
    /// is drawn — not the line, and not its heads.
    pub trim_end: f32,
    /// The dashes, if the line is broken into them.
    pub dash: Option<Dash>,
}

impl Stroking {
    /// The whole line, solid.
    pub const WHOLE: Self = Self {
        trim_start: 0.0,
        trim_end: 1.0,
        dash: None,
    };

    /// The span to draw, clamped to the outline — `None` when it is empty.
    pub(crate) fn span(&self) -> Option<(f32, f32)> {
        let clamp = |fraction: f32, otherwise: f32| {
            if fraction.is_nan() {
                otherwise
            } else {
                fraction.clamp(0.0, 1.0)
            }
        };
        let (start, end) = (clamp(self.trim_start, 0.0), clamp(self.trim_end, 1.0));
        (end > start).then_some((start, end))
    }
}

impl Default for Stroking {
    fn default() -> Self {
        Self::WHOLE
    }
}

/// A line broken into dashes, in pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Dash {
    /// On, off, on, off… — lengths along the line. An odd count is read twice
    /// over, so `[a]` is dashes and gaps of one length and `[a, b, c]` is
    /// `[a, b, c, a, b, c]`: the reading SVG gives it, and the one that means
    /// every entry is used as written.
    pub pattern: Vec<f32>,
    /// How far the pattern has moved along the line, toward its end.
    /// Increasing it makes the dashes flow from the line's start to its end —
    /// from an arrow's tail to its head, and clockwise round a box.
    pub offset: f32,
}

impl Dash {
    /// The pattern tiny-skia takes: an even number of lengths, every one of
    /// them positive and finite, or `None` for a pattern that could not break
    /// a line — which then draws solid, since a line that vanished would look
    /// like a shape that failed to render.
    pub(crate) fn pattern(&self) -> Option<Vec<f32>> {
        if self.pattern.is_empty()
            || self
                .pattern
                .iter()
                .any(|length| !length.is_finite() || *length <= 0.0)
        {
            return None;
        }
        let mut pattern = self.pattern.clone();
        if pattern.len() % 2 == 1 {
            pattern.extend_from_within(..);
        }
        Some(pattern)
    }
}
