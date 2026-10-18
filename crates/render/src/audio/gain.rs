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
/// **Every `volume` track multiplies** (#1099): a hand-written fade and the dip
/// `duck_music` signs are two multipliers on one clip, and both are heard — a
/// fade to half under a duck to a quarter plays at an eighth. Gain is
/// multiplicative, so this is the composition an editor expects, and it is
/// why `volume` is the one path validation lets a clip animate twice
/// ([`PropertyPath::STACKING`](scorsese_core::PropertyPath::STACKING)).
///
/// Holds the clip's tracks rather than a value because gain is evaluated per
/// sample. A clip carries a handful of tracks, so picking out the volume ones
/// at each instant costs nothing beside the multiply.
#[derive(Debug, Clone, Copy)]
pub struct Gain<'a> {
    tracks: &'a [KeyframeTrack],
}

impl<'a> Gain<'a> {
    /// The volume of a clip with these keyframes, whichever of them animate it.
    pub fn of(tracks: &'a [KeyframeTrack]) -> Self {
        Self { tracks }
    }

    /// The tracks that animate volume.
    fn volume(&self) -> impl Iterator<Item = &'a KeyframeTrack> {
        self.tracks
            .iter()
            .filter(|track| track.property.as_str() == path::VOLUME)
    }

    /// True when this clip plays at its recorded level throughout, which lets
    /// the mixer add samples without multiplying them.
    pub fn is_unity(&self) -> bool {
        self.volume().next().is_none()
    }

    /// The multiplier at `t` frames into the clip, where `t` may fall between
    /// two frames: the product of every volume track's own.
    ///
    /// **Interpolated across the frame grid, not stepped onto it.** Keyframes
    /// are control points and the value travels continuously between them, so a
    /// fade evaluated per sample has to travel continuously too. Snapping each
    /// sample to its nearest whole frame would turn a smooth ramp into 30 steps
    /// a second — inaudible as pitch, audible as a zipper.
    ///
    /// Negative values are clamped away, track by track: a multiplier below
    /// zero is a phase inversion, which is not what anyone means by dragging a
    /// volume line below the floor — and two of them must not multiply back
    /// into a gain.
    pub fn at(&self, t: f64) -> f32 {
        let whole = t.max(0.0).floor();
        let fraction = (t - whole).clamp(0.0, 1.0);
        let frame = Frames(whole as u64);
        self.volume()
            .map(|track| {
                match (track.value_at(frame), track.value_at(frame + Frames(1))) {
                    (Some(from), Some(to)) => ((from + (to - from) * fraction) as f32).max(0.0),
                    // An empty track animates nothing, so the clip keeps its
                    // own level rather than being silenced by a property
                    // nobody set.
                    _ => UNITY,
                }
            })
            .product()
    }
}
