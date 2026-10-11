//! Where the points of each dip land.

use crate::keyframe::{Easing, Keyframe, KeyframeTrack, PropertyPath};
use crate::time::Frames;
use crate::timeline::Clip;

use super::{Dip, Span, TOOL, merge};

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
pub(crate) fn plan(
    clip: &Clip,
    covering: &[Span],
    property: PropertyPath,
    dip: Dip,
) -> Option<KeyframeTrack> {
    let spans = merge(covering, dip.gap_worth_keeping());
    if spans.is_empty() {
        return None;
    }
    // A ramp of no frames is still drawn over one: two points on one frame
    // are one point, and the dip it belonged to would lose its fall or its
    // climb rather than become a switch.
    let attack = dip.attack.max(Frames(1));
    let release = dip.release.max(Frames(1));

    let mut keyframes = Vec::with_capacity(spans.len() * 4);
    let mut fall = attack;
    for (i, span) in spans.iter().enumerate() {
        let (rise, next_fall) = match spans.get(i + 1) {
            Some(next) => share(Frames(next.from.get() - span.to.get()), attack, release),
            None => (release, attack),
        };
        push_dip(
            &mut keyframes,
            span,
            clip.duration,
            dip,
            Ramps { fall, rise },
        );
        fall = next_fall;
    }
    // Two spans close enough to share a keyframe would otherwise leave a
    // repeated time, which is exactly what validation rejects. Merging by
    // `gap_worth_keeping` makes that rare rather than impossible — a span that
    // starts at frame 0 has no room for its attack.
    keyframes.dedup_by_key(|keyframe| keyframe.t);
    Some(KeyframeTrack::new(property, keyframes).generated_by(TOOL))
}

/// How long one dip takes to fall into its span and to climb out of it.
#[derive(Clone, Copy)]
struct Ramps {
    fall: Frames,
    rise: Frames,
}

/// How a pause of `gap` frames between two dips is shared between the climb
/// out of the first (`.0`) and the fall into the second (`.1`).
///
/// A pause with room for both ramps keeps them as asked. A shorter one — only
/// possible when [`Dip::min_gap`] keeps pauses the default would merge — is
/// split between them in proportion, so the climb ends on the very frame the
/// fall begins. `gap` is at least two frames, which merging guarantees, so
/// each side gets one at least.
fn share(gap: Frames, attack: Frames, release: Frames) -> (Frames, Frames) {
    let both = attack.get() + release.get();
    let gap = gap.get();
    if gap >= both {
        return (release, attack);
    }
    // Rounded to the nearest frame, in integers.
    let rise = ((2 * gap * release.get() + both) / (2 * both)).clamp(1, gap - 1);
    (Frames(rise), Frames(gap - rise))
}

/// The four points of one dip, clamped into the clip.
fn push_dip(into: &mut Vec<Keyframe>, span: &Span, duration: Frames, dip: Dip, ramps: Ramps) {
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
    let start = Frames(span.from.get().saturating_sub(ramps.fall.get()));
    if start > Frames::ZERO || span.from > Frames::ZERO {
        // A pause shared between two ramps ends one dip's climb on the frame
        // the next one's fall begins. That point is this one: it carries the
        // fall's easing, so it replaces the climb's rather than being dropped.
        if into.last().is_some_and(|last| last.t == start) {
            into.pop();
        }
        into.push(full(start, Easing::EaseOut));
    }
    into.push(dipped(span.from, Easing::Linear));
    let end = span.to.min(duration);
    into.push(dipped(end, Easing::EaseIn));
    into.push(full((end + ramps.rise).min(duration), Easing::Linear));
}
