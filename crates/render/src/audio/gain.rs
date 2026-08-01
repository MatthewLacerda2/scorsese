//! Where the `volume` property path acquires meaning.
//!
//! `scorsese-core` knows only that a keyframe track animates *some* numeric
//! property. `scorsese-compositor` is where the visual paths — `opacity`,
//! `transform.scale.x` — become real; this module is the same thing for sound.
//! Each renderer of a property owns its own name, next to the code that
//! implements it, which is what keeps the format ignorant of both.
//!
//! There is no audio-specific keyframe machinery here, and that is the point: a
//! volume ramp is evaluated by the same [`KeyframeTrack::value_at`] that
//! evaluates a fade.

use scorsese_compositor::Property;
use scorsese_core::{Frames, KeyframeTrack};

/// The property paths the mixer resolves.
pub mod path {
    /// How loud a clip plays, as a multiplier: `1.0` is the source untouched,
    /// `0.0` is silence, and above `1.0` is gain.
    pub const VOLUME: &str = "volume";
}

/// What the mixer animates.
///
/// Its own list rather than an entry in the compositor's, for the same reason
/// the constant above is here: each renderer of a property owns its name next
/// to the code that implements it. A `volume` listed in a crate that draws
/// pictures would be a name with no implementation behind it, which is the
/// drift a published vocabulary exists to prevent.
pub(crate) const ANIMATED: &[Property] = &[Property {
    path: path::VOLUME,
    describes: "how loud the clip plays, as a multiplier on its own level",
}];

/// Volume with no keyframes: the source exactly as it was recorded.
pub(crate) const UNITY: f32 = 1.0;

/// A clip's volume over time, ready to be asked about any instant.
///
/// Holds the track rather than a value because gain is evaluated per sample,
/// and looking the property up once per clip beats looking it up 48,000 times a
/// second.
#[derive(Debug, Clone, Copy)]
pub struct Gain<'a> {
    track: Option<&'a KeyframeTrack>,
}

impl<'a> Gain<'a> {
    /// Finds the volume track among a clip's keyframes, if it has one.
    pub fn of(tracks: &'a [KeyframeTrack]) -> Self {
        Self {
            track: tracks
                .iter()
                .find(|track| track.property.as_str() == path::VOLUME),
        }
    }

    /// True when this clip plays at its recorded level throughout, which lets
    /// the mixer add samples without multiplying them.
    pub fn is_unity(&self) -> bool {
        self.track.is_none()
    }

    /// The multiplier at `t` frames into the clip, where `t` may fall between
    /// two frames.
    ///
    /// **Interpolated across the frame grid, not stepped onto it.** Keyframes
    /// are control points and the value travels continuously between them, so a
    /// fade evaluated per sample has to travel continuously too. Snapping each
    /// sample to its nearest whole frame would turn a smooth ramp into 30 steps
    /// a second — inaudible as pitch, audible as a zipper.
    ///
    /// Negative values are clamped away: a multiplier below zero is a phase
    /// inversion, which is not what anyone means by dragging a volume line
    /// below the floor.
    pub fn at(&self, t: f64) -> f32 {
        let Some(track) = self.track else {
            return UNITY;
        };
        let whole = t.max(0.0).floor();
        let fraction = (t - whole).clamp(0.0, 1.0);
        let frame = Frames(whole as u64);
        let (from, to) = (track.value_at(frame), track.value_at(frame + Frames(1)));
        match (from, to) {
            (Some(from), Some(to)) => ((from + (to - from) * fraction) as f32).max(0.0),
            // An empty track animates nothing, so the clip keeps its own level
            // rather than being silenced by a property nobody set.
            _ => UNITY,
        }
    }
}
