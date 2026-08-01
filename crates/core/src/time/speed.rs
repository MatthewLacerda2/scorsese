//! How fast a clip runs against the timeline it sits on.

use serde::{Deserialize, Serialize};

use super::frames::Frames;

/// The rate a clip's source plays at, against the timeline grid.
///
/// `1.0` is the source's own rate — one source frame per timeline frame, which
/// is what every clip did before this field existed. `2.0` plays it twice as
/// fast, so one timeline frame advances two frames into the source; `0.5` plays
/// it half as fast and a source frame is held for two.
///
/// **It is a rate, never a length.** `duration` still says how many frames of
/// the *timeline* a clip occupies, and the format deliberately does not couple
/// the two. Picking 2× in an editor usually also halves the clip on the
/// timeline — the same footage in half the time — but that is
/// [`crate::Clip::retime`], an operation a GUI or an assistant performs, not
/// something the document does behind an author's back. Keeping the split means
/// a clip can be sped up *and* trimmed afterwards without two fields fighting
/// over one fact.
///
/// **A speed change takes the pitch of the sound with it.** At 2× the audio is
/// resampled and it rises, which is what anyone who has used a consumer editor
/// expects and what the arithmetic gives for nothing. Preserving pitch is a
/// time-stretch algorithm, a new dependency and its own set of artefacts — a
/// separate piece of work rather than a flag hanging off this one.
///
/// **Constant over the clip.** A speed that is itself keyframed — an
/// acceleration within a shot — turns the map from timeline frame to source
/// frame from a multiplication into an integral, which is a different and much
/// larger problem. So is a negative speed, which would need the decoder to seek
/// rather than stream. Neither is expressible here, on purpose.
///
/// Nothing checks the number as it is read: any float parses, and a zero,
/// negative or non-finite one is reported by [`crate::validate`] naming the clip
/// that carries it. That is the same bargain every other field makes — an editor
/// may save a document that is briefly incoherent and hear about all of it at
/// once — and it is why this is not a `try_from` on the wire.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Speed(f64);

impl Speed {
    /// The source's own rate: one source frame per timeline frame. What a clip
    /// saying nothing means, and what every clip meant before there was a field
    /// to say it with.
    pub const NORMAL: Self = Self(1.0);

    /// Wraps a rate as written. Unchecked, so that a bad one is a validation
    /// finding that names its clip rather than a parse failure that names a
    /// line.
    pub const fn new(rate: f64) -> Self {
        Self(rate)
    }

    /// The rate as a plain float — for arithmetic, and for the ffmpeg filters
    /// that take one.
    pub const fn get(self) -> f64 {
        self.0
    }

    /// True for [`Speed::NORMAL`], which is what an absent field means. Keeps
    /// the field out of the documents — nearly all of them — that never set it.
    pub fn is_normal(&self) -> bool {
        self.0 == 1.0
    }

    /// True when this is a rate something could actually play at.
    ///
    /// Zero is a clip that never advances through its source, a negative is
    /// playing backwards, and neither infinity nor NaN is a rate at all. Every
    /// one of them would reach a decoder as a filter argument nothing sensible
    /// comes back from.
    pub fn is_usable(self) -> bool {
        self.0.is_finite() && self.0 > 0.0
    }

    /// How far into its source a stretch of timeline carries, in frames of the
    /// timeline's own grid.
    ///
    /// Fractional, and that is the point: at any speed but a whole one the
    /// answer lands between two frames, and rounding it before it reaches a
    /// decoder is how a segment resumed after a cut on another track ends up
    /// half a frame from where the one before it stopped.
    pub fn source_frames(self, timeline: Frames) -> f64 {
        timeline.get() as f64 * self.0
    }

    /// How much timeline it takes to play a stretch of source — the inverse of
    /// [`Speed::source_frames`], and only meaningful for a rate
    /// [`Speed::is_usable`] accepts.
    pub fn timeline_frames(self, source: f64) -> f64 {
        source / self.0
    }
}

impl Default for Speed {
    fn default() -> Self {
        Self::NORMAL
    }
}

impl std::fmt::Display for Speed {
    /// `2×`, `0.5×` — the way a speed is written on a menu, so a render's
    /// description reads as an edit rather than as a field dump.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}×", self.0)
    }
}
