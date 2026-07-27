//! What a clip's keyframes actually do over one stretch of timeline.
//!
//! A fade is the thing a description most has to get right, and "there is an
//! opacity track on this clip" is not it: the questions a reader has are which
//! way it goes and when, and a description that answers neither would pass a
//! render whose fade runs backwards or lands two seconds early.
//!
//! So each property is read three ways — what it is at the top of the stretch,
//! what it is at the bottom, and where in between it travels. Keyframes never
//! cut the timeline, so the travel is a span *inside* a stretch rather than a
//! stretch of its own.

use scorsese_compositor::Property;
use scorsese_core::{Clip, Easing, Fps, Frames, Keyframe, KeyframeTrack};

use super::Moment;

/// One animated property, as it reads across one stretch.
#[derive(Debug, Clone, PartialEq)]
pub struct Animated {
    /// The property path, as the document writes it.
    pub property: String,
    /// What it reads at the stretch's first frame.
    pub at_start: f64,
    /// What it reads at the stretch's **end boundary** — the frame after its
    /// last one, not that last frame.
    ///
    /// Deliberately: a fade written to reach zero exactly on the cut has its
    /// final keyframe at the clip's end, so the boundary is where the value
    /// the author wrote actually is. Reading the last rendered frame instead
    /// would report a fade to 0.03 and make every clean fade look like a
    /// rounding error.
    pub at_end: f64,
    /// Where inside the stretch the value moves. `None` when it holds.
    pub travel: Option<Travel>,
}

impl Animated {
    /// True when this property is zero across the whole stretch and never
    /// moves — a clip muted rather than merely quiet.
    pub fn is_zero(&self) -> bool {
        self.travel.is_none() && self.at_start == 0.0 && self.at_end == 0.0
    }
}

/// The span of a stretch over which an animated value is in motion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Travel {
    /// Where the movement starts, clamped to the stretch it is reported in.
    pub from: Moment,
    /// Where it finishes, clamped the same way.
    pub to: Moment,
    /// How the value travels. `None` when the legs inside this stretch do not
    /// agree on one curve, which is the honest answer rather than naming the
    /// first of them.
    pub easing: Option<Easing>,
}

/// Every property of `clip` that `vocabulary` animates, read across
/// `from..to` on the timeline.
///
/// A keyframe track naming something no vocabulary knows is left out. It will
/// never do anything, [`crate::properties::unknown_in`] says so on the render
/// report, and repeating it here as though it were an animation would be the
/// description asserting a fade that does not exist.
pub(super) fn across(
    clip: &Clip,
    vocabulary: &[Property],
    from: Frames,
    to: Frames,
    fps: Fps,
) -> Vec<Animated> {
    clip.keyframes
        .iter()
        .filter(|track| {
            vocabulary
                .iter()
                .any(|property| property.path == track.property.as_str())
        })
        .filter_map(|track| one(track, clip, from, to, fps))
        .collect()
}

/// One track, or `None` when it holds no keyframes and so animates nothing.
fn one(track: &KeyframeTrack, clip: &Clip, from: Frames, to: Frames, fps: Fps) -> Option<Animated> {
    // Keyframe times are relative to the clip's own start, which is what lets a
    // clip be moved along the timeline without rewriting them.
    let relative = |at: Frames| Frames(at.get().saturating_sub(clip.start.get()));
    Some(Animated {
        property: track.property.to_string(),
        at_start: track.value_at(relative(from))?,
        at_end: track.value_at(relative(to))?,
        travel: travel(track, clip, from, to, fps),
    })
}

/// The union of the legs that move inside `from..to`.
fn travel(
    track: &KeyframeTrack,
    clip: &Clip,
    from: Frames,
    to: Frames,
    fps: Fps,
) -> Option<Travel> {
    let mut span: Option<(Frames, Frames)> = None;
    let mut easings: Vec<Easing> = Vec::new();
    for pair in track.keyframes.windows(2) {
        let Some((start, end)) = leg(pair[0], pair[1], clip) else {
            continue;
        };
        let (low, high) = (start.max(from), end.min(to));
        if low >= high {
            continue;
        }
        span = Some(match span {
            Some((was_low, was_high)) => (was_low.min(low), was_high.max(high)),
            None => (low, high),
        });
        easings.push(pair[0].easing);
    }
    let (low, high) = span?;
    Some(Travel {
        from: Moment::at(low, fps),
        to: Moment::at(high, fps),
        easing: match easings.first() {
            Some(&first) if easings.iter().all(|&easing| easing == first) => Some(first),
            _ => None,
        },
    })
}

/// Where on the timeline one leg between two keyframes moves, or `None` when
/// it does not move at all.
///
/// A `hold` leg is the exception worth spelling out: the value does not travel
/// across it, it sits still and then jumps at the far keyframe. So its movement
/// is the single frame it lands on, not the span it waited out.
fn leg(from: Keyframe, to: Keyframe, clip: &Clip) -> Option<(Frames, Frames)> {
    if from.value == to.value {
        return None;
    }
    let landing = clip.start + to.t;
    if from.easing == Easing::Hold {
        return Some((landing, landing + Frames(1)));
    }
    Some((clip.start + from.t, landing))
}
