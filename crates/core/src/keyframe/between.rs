//! Where an instant falls on a keyframe track, before any easing is applied.
//!
//! [`KeyframeTrack::value_at`] is the answer nearly every property wants: one
//! number, eased. A few want the parts it is made of instead — the two values
//! either side of the instant, how far from one to the other, and the easing
//! that would have been applied — because what they do with the curve is not
//! "evaluate it once". A text reveal hands the easing to each word rather than
//! to the whole line, and a counter wants to know the span it is between so
//! that an overshooting curve cannot put a figure on screen nobody wrote.
//!
//! **Still generic.** Nothing here knows which property is asking; it is the
//! same walk over the same keyframes, stopped one step earlier.

use crate::time::Frames;

use super::{Easing, KeyframeTrack};

/// The stretch of a track an instant falls in, uneased.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Between {
    /// The value at the keyframe the stretch leaves.
    pub from: f64,
    /// The value at the keyframe it arrives at. Equal to `from` outside the
    /// keyframed span, where the value holds.
    pub to: f64,
    /// How far along the stretch the instant is, `0.0` at `from` and `1.0` at
    /// `to` — linear in time, before any easing.
    pub progress: f64,
    /// The easing the stretch carries, which is the one on the keyframe it
    /// leaves. Outside the keyframed span it is the easing of the stretch
    /// nearest: the first one before the track starts, the last one after it
    /// ends — so a property that reads it sees no jump at either end.
    pub easing: Easing,
}

impl Between {
    /// The value this stretch has at its progress, eased — exactly what
    /// [`KeyframeTrack::value_at`] answers.
    pub fn eased(&self) -> f64 {
        self.from + (self.to - self.from) * self.easing.apply(self.progress)
    }

    /// The value it would have if the stretch were linear.
    pub fn linear(&self) -> f64 {
        self.from + (self.to - self.from) * self.progress.clamp(0.0, 1.0)
    }
}

impl KeyframeTrack {
    /// Where `t` falls on this track: the stretch it is in and how far along.
    ///
    /// `None` only when the track holds no keyframes, as for
    /// [`KeyframeTrack::value_at`], which is this followed by
    /// [`Between::eased`].
    pub fn between(&self, t: Frames) -> Option<Between> {
        let first = self.keyframes.first()?;
        let last = self.keyframes.last()?;
        let held = |value: f64, progress: f64, easing: Easing| Between {
            from: value,
            to: value,
            progress,
            easing,
        };
        if t <= first.t {
            return Some(held(first.value, 0.0, first.easing));
        }
        if t >= last.t {
            // The stretch that arrived here, when there is one — its easing is
            // the one a reader was already using a frame ago.
            let arriving = self.keyframes.iter().rev().nth(1).unwrap_or(last);
            return Some(held(last.value, 1.0, arriving.easing));
        }
        let pair = self
            .keyframes
            .windows(2)
            .find(|pair| pair[0].t <= t && t < pair[1].t)?;
        let (from, to) = (&pair[0], &pair[1]);
        let span = to.t.get().saturating_sub(from.t.get());
        if span == 0 {
            return Some(held(from.value, 0.0, from.easing));
        }
        Some(Between {
            from: from.value,
            to: to.value,
            progress: (t.get() - from.t.get()) as f64 / span as f64,
            easing: from.easing,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyframe::{Keyframe, PropertyPath};

    fn track(keys: &[(u64, f64, Easing)]) -> KeyframeTrack {
        KeyframeTrack::new(
            PropertyPath::new("reveal"),
            keys.iter()
                .map(|(t, value, easing)| Keyframe {
                    t: Frames::from(*t),
                    value: *value,
                    easing: *easing,
                })
                .collect(),
        )
    }

    #[test]
    fn mid_stretch_is_linear_progress_and_the_leaving_keyframe_s_easing() {
        let track = track(&[(0, 0.0, Easing::BackOut), (10, 1.0, Easing::Linear)]);
        let between = track.between(Frames::from(5)).expect("keyframes");
        assert_eq!(between.progress, 0.5);
        assert_eq!(between.easing, Easing::BackOut);
        assert_eq!(between.linear(), 0.5);
        assert_eq!(Some(between.eased()), track.value_at(Frames::from(5)));
    }

    #[test]
    fn past_the_end_holds_with_the_arriving_stretch_s_easing() {
        let track = track(&[(0, 0.0, Easing::EaseOut), (10, 2.0, Easing::Hold)]);
        let after = track.between(Frames::from(40)).expect("keyframes");
        assert_eq!((after.from, after.to, after.progress), (2.0, 2.0, 1.0));
        assert_eq!(after.easing, Easing::EaseOut);
        let before = track.between(Frames::from(0)).expect("keyframes");
        assert_eq!((before.from, before.to), (0.0, 0.0));
        assert_eq!(before.easing, Easing::EaseOut);
    }

    #[test]
    fn a_track_with_nothing_in_it_is_nowhere() {
        assert_eq!(track(&[]).between(Frames::from(3)), None);
    }
}
