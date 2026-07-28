//! Lowering one clip's property while other clips are playing.
//!
//! This is the mechanism behind **auto-ducking** — music stepping aside for
//! narration — but it is written without ever naming `volume`, and that is
//! deliberate.
//!
//! `scorsese-core` defines property *types*, never property *values*, and the
//! meaning of `volume` belongs to the mixer that implements it. So a caller
//! hands in the [`PropertyPath`] to dip and the two levels to dip between, and
//! this module does the arithmetic: which spans of a clip are covered, which of
//! those are close enough to be one dip rather than two, and where the ramps
//! land.
//!
//! What comes out is an ordinary [`KeyframeTrack`] — the same thing a person
//! would have placed by hand, evaluated by the same [`KeyframeTrack::value_at`]
//! that evaluates a fade. There is no ducking stage anywhere in the mixer, and
//! there should never be one.
//!
//! ```text
//!      ────────╮                          ╭────────   full
//!               ╲                        ╱
//!                ╰──────────────────────╯             dipped
//!        attack  │      covered span    │  release
//! ```

mod apply;
mod spans;

use crate::keyframe::{Easing, Keyframe, KeyframeTrack, PropertyPath};
use crate::time::Frames;
use crate::timeline::{Clip, Track, TrackKind};

pub use apply::{Ducked, Under, audio_tracks, duck_track};
pub use spans::{Span, covering, merge};

/// Who signs the keyframe tracks this module produces.
///
/// A constant rather than a caller's choice, because the whole value of the
/// signature is that the *next* run recognises it — a name that varies is a
/// name that never matches.
pub const TOOL: &str = "duck";

/// How far the property dips, and how long it takes to get there and back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dip {
    /// The value held while something is covering. For volume, a multiplier:
    /// `0.25` is a quarter as loud.
    pub under: f64,
    /// The value everywhere else — what the clip reads when nothing covers it.
    pub over: f64,
    /// How long the fall takes, in frames. The dip is fully down *by* the
    /// moment the covering clip starts, not after it.
    pub attack: Frames,
    /// How long the climb back takes, in frames.
    pub release: Frames,
}

impl Dip {
    /// A quarter-level dip with a fast fall and a slower recovery, on a grid
    /// of `fps` frames per second.
    ///
    /// The asymmetry is the point and it is how every ducking compressor ever
    /// built behaves: getting out of the way late is audible as a word being
    /// stepped on, and coming back early is audible as a lurch. So the fall is
    /// quick and the recovery is unhurried.
    pub fn default_for(fps: f64) -> Self {
        Self {
            under: 0.25,
            over: 1.0,
            attack: Frames((fps * 0.3).round() as u64),
            release: Frames((fps * 0.6).round() as u64),
        }
    }

    /// The shortest gap between two covered spans worth treating as a gap.
    ///
    /// Two lines of narration a beat apart should be one dip, not two: coming
    /// all the way back up and immediately going down again is pumping, and it
    /// draws more attention than the thing it was avoiding. Anything closer
    /// than a full recovery and fall is merged.
    pub fn gap_worth_keeping(self) -> Frames {
        self.release + self.attack
    }
}

/// The keyframes that hold `property` at [`Dip::over`], dipping to
/// [`Dip::under`] across every span of `clip` that `covering` clips occupy.
///
/// `None` when nothing covers the clip — a track with no keyframes is not the
/// same as a track that says "stay at full", and writing the second would be
/// claiming an animation nobody asked for.
///
/// Times are clip-relative, like every keyframe: moving the clip later does
/// not rewrite them, and it does not need to, because the spans were computed
/// against where the clip is now.
pub fn plan(
    clip: &Clip,
    covering: &[Span],
    property: PropertyPath,
    dip: Dip,
) -> Option<KeyframeTrack> {
    let spans = merge(covering, dip.gap_worth_keeping());
    if spans.is_empty() {
        return None;
    }
    let mut keyframes = Vec::with_capacity(spans.len() * 4);
    for span in &spans {
        push_dip(&mut keyframes, span, clip.duration, dip);
    }
    // Two spans close enough to share a keyframe would otherwise leave a
    // repeated time, which is exactly what validation rejects. Merging by
    // `gap_worth_keeping` makes that rare rather than impossible — a span that
    // starts at frame 0 has no room for its attack.
    keyframes.dedup_by_key(|keyframe| keyframe.t);
    Some(KeyframeTrack::new(property, keyframes).generated_by(TOOL))
}

/// The four points of one dip, clamped into the clip.
fn push_dip(into: &mut Vec<Keyframe>, span: &Span, duration: Frames, dip: Dip) {
    let full = |t: Frames, easing| Keyframe {
        t,
        value: dip.over,
        easing,
    };
    let dipped = |t: Frames, easing| Keyframe {
        t,
        value: dip.under,
        easing,
    };
    // `ease_out` on the way down and `ease_in` on the way back: the level
    // leaves and returns to full gently, and passes through the middle of each
    // ramp quickly, which is what stops a duck sounding like a switch.
    let start = Frames(span.from.get().saturating_sub(dip.attack.get()));
    if start > Frames::ZERO || span.from > Frames::ZERO {
        into.push(full(start, Easing::EaseOut));
    }
    into.push(dipped(span.from, Easing::Linear));
    let end = span.to.min(duration);
    into.push(dipped(end, Easing::EaseIn));
    into.push(full((end + dip.release).min(duration), Easing::Linear));
}

/// True when this track is one whose clips *cover* others — the narration side
/// of the arrangement rather than the music side.
///
/// Audio only: dipping a picture under another picture is a real effect, but
/// it is not this one, and a caller that wants it should say which tracks
/// rather than have it guessed.
pub fn is_coverable(track: &Track) -> bool {
    track.kind == TrackKind::Audio
}
