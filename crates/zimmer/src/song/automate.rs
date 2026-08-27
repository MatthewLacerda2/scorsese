//! A value that moves across the piece.
//!
//! Everything else in a recipe is one number for the whole piece: a track's
//! `gain`, a filter's `cutoff`. Envelopes and the LFO move things *within a
//! note*, which is a different scale entirely — so a **build**, the filter
//! opening and the level rising over eight bars into a drop, was unwritable,
//! and so were a riser, a fade-in on one instrument and a lead getting
//! brighter as the piece intensifies. What could be written was a section
//! louder than the one before it, which is a step, not a build.
//!
//! ## The same shape as a keyframe, in beats
//!
//! `scorsese-core` animates a clip with `(property, [(t, value, easing)])`,
//! and this is that shape one crate over: a list of points, a value between
//! them, and the same five easing curves under the same five names. The two
//! cannot share code — this crate has never heard of `core` — but they must
//! not disagree about what `ease_in_out` means, so they do not.
//!
//! **Time is beats**, like everything else in a song, so changing the tempo of
//! a piece with a build in it is still one number.
//!
//! ## Which properties, and why it is a closed list
//!
//! `core`'s property path is a string, deliberately: a path nothing animates
//! is ignored and warned about, because a project written against a newer
//! build still has to render on an older one. Nothing in this crate works that
//! way. A song is refused for a typo'd track or pattern name rather than
//! rendered with a hole in it, because *silence in the middle of a piece* is
//! the failure that costs an agent a whole iteration to even notice — and an
//! automation curve that moves nothing is exactly that failure, quieter.
//!
//! So [`Param`] is a closed enum. A misspelled parameter is refused by serde,
//! against a list of the words that work, before anything is rendered. A curve
//! on a track that does not exist, or a `cutoff` curve on an instrument with
//! no filter, is refused too — see the song's validation.
//!
//! ## Where it is sampled, which is not one answer
//!
//! A curve is sampled **where the thing it moves is decided**, and the two
//! parameters this touches decide at different rates:
//!
//! - [`Param::Gain`] and [`Param::Pan`] are faders. A fader is a property of
//!   the mix at every instant, so they are applied **per sample**, on the
//!   track's own bus, after its chain. That costs one multiply per sample per
//!   automated track and it is *correct*: a pad holding one chord through the
//!   whole build actually swells, which a per-note answer could not do.
//! - [`Param::Cutoff`] is a voice parameter. A note is rendered as one buffer
//!   through a filter whose cutoff is set when the voice starts, so a curve is
//!   sampled **once per note onset** and holds for that note. Per sample would
//!   mean a time-varying cutoff threaded through the filter and its envelope —
//!   a change to every note rendered in the crate, for a difference audible
//!   only *inside* one note.
//!
//! The honest limit of that second choice: a filter sweep is as smooth as the
//! part playing it. An arpeggio at a sixteenth gets 32 steps a bar, which is a
//! sweep; one held chord across eight bars gets one value, which is not. The
//! fix is to write the part as the repeated notes it would be played as.
//!
//! ## Under `fit`
//!
//! Beats are counted from the start of the **rendered piece**, once, and a
//! curve never repeats.
//!
//! - `stretch` moves the tempo and leaves the beats alone, so the build
//!   stretches with the music and stays a build over the same eight bars.
//! - `loop` repeats the *arrangement*, and the curve does not go back with it:
//!   a build that restarted every pass would be a saw. A piece looping to
//!   45 seconds can carry a curve across all of it, because the beats keep
//!   counting — which is the general answer, and the reason nothing has to be
//!   refused here.
//! - `once` plays through and pads with silence; the curve simply ends with
//!   the music.

use serde::{Deserialize, Serialize};

use super::{Song, Track};
use crate::stereo::{self, Stereo};

/// What a curve moves.
///
/// Closed and small, for the reason the module doc gives: a wrong word is a
/// refusal here rather than a curve that quietly moves nothing. Each is a
/// number that was already a field of the document and was already constant
/// for the whole piece.
///
/// **An fx parameter is deliberately not here yet.** Every effect has
/// different fields, so naming one is a path into a chain rather than a word —
/// a second design — and a chain runs once over a whole bus, so honouring it
/// would mean a time-varying parameter inside every effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Param {
    /// The track's linear [`gain`](Track::gain) — the fader. A part rising
    /// into a chorus, a riser, a fade-in on one instrument.
    Gain,
    /// The track's [`pan`](Track::pan) — a part drifting across the image.
    /// Clamped to `-1.0..=1.0` at every instant, as a written `pan` is.
    Pan,
    /// The cutoff of the instrument's filter, in Hz — the build. Refused on a
    /// track whose patch has no filter, since there would be nothing to move.
    Cutoff,
}

impl Param {
    /// The word a recipe spells it with — what a refusal names it by.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gain => "gain",
            Self::Pan => "pan",
            Self::Cutoff => "cutoff",
        }
    }
}

/// How a value approaches the next point.
///
/// The same five, under the same names and with the same arithmetic, as
/// `scorsese_core::keyframe::Easing`. Written out again rather than shared,
/// because this crate does not depend on that one — but *chosen* rather than
/// invented, so an agent that has written a clip's keyframes already knows
/// these.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Easing {
    /// Constant rate throughout, and what a point that says nothing means.
    #[default]
    Linear,
    /// Starts slow and accelerates into the next point.
    EaseIn,
    /// Starts at full rate and settles as it arrives.
    EaseOut,
    /// Slow at both ends, quickest in the middle — the one that reads as
    /// deliberate rather than mechanical.
    EaseInOut,
    /// Holds this value until the next point, then jumps. A step, written on
    /// purpose.
    Hold,
}

impl Easing {
    /// Reshapes linear progress through a segment, `0.0..=1.0`, into eased
    /// progress.
    pub fn apply(self, progress: f32) -> f32 {
        let p = progress.clamp(0.0, 1.0);
        match self {
            Self::Linear => p,
            Self::EaseIn => p * p,
            Self::EaseOut => 1.0 - (1.0 - p) * (1.0 - p),
            // Smoothstep: symmetric, and flat at both ends.
            Self::EaseInOut => p * p * (3.0 - 2.0 * p),
            // The value does not travel at all; it jumps at the next point.
            Self::Hold => 0.0,
        }
    }
}

/// One control point: a beat, what the value reads there, and how it travels
/// from there to the next.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Point {
    /// Beats from the start of the rendered piece. Not from the start of a
    /// pattern and not from the start of a `loop` pass — see the module doc.
    pub beat: f32,
    /// What the parameter reads at this instant, in the parameter's own units:
    /// linear for `gain`, `-1.0..=1.0` for `pan`, Hz for `cutoff`.
    pub value: f32,
    /// How the value travels from here to the next point.
    #[serde(default, skip_serializing_if = "is_linear")]
    pub easing: Easing,
}

/// Whether a point travels at a constant rate — the test that keeps
/// `"easing": "linear"` out of every saved document, for the reason a song's
/// `swing` is left out when it is zero: a serialiser that started writing a
/// default would invalidate every cached bake for no change in the audio.
fn is_linear(easing: &Easing) -> bool {
    *easing == Easing::Linear
}

/// One parameter of one track, moving across the piece.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Automation {
    /// The [`Track::name`] this rides. It is the *arrangement* that builds, so
    /// this lives on the song and names a track — a patch shared between two
    /// songs must not carry one song's build.
    pub track: String,
    /// Which parameter moves.
    pub param: Param,
    /// The control points, in ascending beat order.
    pub points: Vec<Point>,
}

impl Automation {
    /// What this parameter reads at `beat`.
    ///
    /// `None` only when there are no points at all, which validation refuses —
    /// a curve with nothing to say leaves whatever reads it on the number the
    /// document already wrote, rather than asserting a zero nobody meant.
    ///
    /// Outside the written span the value **holds**: before the first point it
    /// is the first value, after the last it is the last. Extrapolating would
    /// make a two-point build keep climbing forever, which is nobody's reading
    /// of two points.
    pub fn value_at(&self, beat: f32) -> Option<f32> {
        let first = self.points.first()?;
        let last = self.points.last()?;
        if beat <= first.beat {
            return Some(first.value);
        }
        if beat >= last.beat {
            return Some(last.value);
        }
        let pair = self
            .points
            .windows(2)
            .find(|pair| pair[0].beat <= beat && beat < pair[1].beat)?;
        let (from, to) = (&pair[0], &pair[1]);
        let span = to.beat - from.beat;
        if span <= 0.0 {
            return Some(from.value);
        }
        let progress = from.easing.apply((beat - from.beat) / span);
        Some(from.value + (to.value - from.value) * progress)
    }

    /// True when the beats ascend strictly. Checked by validation; everything
    /// that evaluates a curve is entitled to assume it.
    pub(super) fn is_sorted(&self) -> bool {
        self.points
            .windows(2)
            .all(|pair| pair[0].beat < pair[1].beat)
    }
}

/// The curves riding one track, looked up once instead of per note.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Riding<'a> {
    /// The fader's level, if it moves.
    pub(super) gain: Option<&'a Automation>,
    /// The fader's position, if it moves.
    pub(super) pan: Option<&'a Automation>,
    /// The instrument's filter cutoff, if it moves.
    pub(super) cutoff: Option<&'a Automation>,
}

impl Riding<'_> {
    /// Whether anything about this track's placement in the mix moves, which
    /// is what decides whether it needs a bus of its own to ride.
    pub(super) fn moves_the_fader(&self) -> bool {
        self.gain.is_some() || self.pan.is_some()
    }
}

/// One entry per track of `song`, in track order.
///
/// Built once per render: the note loop and the fold-down both ask about a
/// track many thousands of times, and neither should be scanning a list of
/// curves to do it.
pub(super) fn riding(song: &Song) -> Vec<Riding<'_>> {
    song.tracks
        .iter()
        .map(|track| {
            let mut riding = Riding::default();
            for curve in song.automation.iter().filter(|it| it.track == track.name) {
                let slot = match curve.param {
                    Param::Gain => &mut riding.gain,
                    Param::Pan => &mut riding.pan,
                    Param::Cutoff => &mut riding.cutoff,
                };
                *slot = Some(curve);
            }
            riding
        })
        .collect()
}

/// Applies a track's moving fader to its own bus, sample by sample.
///
/// The written `gain` and `pan` are what a curve that does not exist leaves
/// behind, so a track that automates only its level keeps the position it was
/// written at, and the other way round.
///
/// `beats_per_sample` comes from the tempo the piece is actually rendered at,
/// which is the stretched one under a `stretch` fit — that is what makes a
/// build stretch with the music rather than land somewhere else in it.
pub(super) fn ride(bus: &mut Stereo, track: &Track, riding: Riding<'_>, beats_per_sample: f32) {
    let fixed = stereo::pan_gains(track.pan);
    for frame in 0..bus.frames() {
        let beat = frame as f32 * beats_per_sample;
        let level = riding
            .gain
            .and_then(|curve| curve.value_at(beat))
            .unwrap_or(track.gain);
        let (left, right) = match riding.pan {
            Some(curve) => stereo::pan_gains(curve.value_at(beat).unwrap_or(track.pan)),
            None => fixed,
        };
        bus.l[frame] *= level * left;
        bus.r[frame] *= level * right;
    }
}

/// Which curve puts a track on a bus of its own.
///
/// A mix asserts what it sounds like, and that is exactly the wrong instrument
/// for this: whether a track is bussed changes the *rounding* of a sum and
/// nothing else, so a predicate stuck at "yes" sounds identical to one that
/// works. It is asked here, by the number.
#[cfg(test)]
mod tests {
    use super::*;

    /// A curve of one point on `param` — enough to occupy a slot.
    fn parked(param: Param) -> Automation {
        Automation {
            track: "pad".to_owned(),
            param,
            points: vec![Point {
                beat: 0.0,
                value: 1.0,
                easing: Easing::Linear,
            }],
        }
    }

    /// The faders, and only the faders. A `cutoff` is read once at a note's
    /// onset and changes nothing about how the part is summed, so a track that
    /// only sweeps its filter keeps the straight-to-master path a track with no
    /// curves at all takes — which is the path the bit-identical promise rests
    /// on.
    #[test]
    fn a_fader_puts_a_track_on_a_bus_and_a_sweep_does_not() {
        let (gain, pan, cutoff) = (
            parked(Param::Gain),
            parked(Param::Pan),
            parked(Param::Cutoff),
        );
        for (moves, riding) in [
            (false, Riding::default()),
            (
                true,
                Riding {
                    gain: Some(&gain),
                    ..Riding::default()
                },
            ),
            (
                true,
                Riding {
                    pan: Some(&pan),
                    ..Riding::default()
                },
            ),
            (
                false,
                Riding {
                    cutoff: Some(&cutoff),
                    ..Riding::default()
                },
            ),
        ] {
            assert_eq!(riding.moves_the_fader(), moves, "{riding:?}");
        }
    }
}
