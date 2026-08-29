//! feedback echo.
//!
//! A ring buffer one echo-time long, read and written each sample: what comes out
//! is fed back in, scaled, so each repeat is quieter than the last. Cheap, and it is
//! the effect that most reliably makes a dry one-shot sound like it happened
//! *somewhere* — a slapback on a gunshot, a corridor on a footstep.
//!
//! The wet signal is blended with the dry one by `mix`, so a delay is always a
//! parallel effect, never a replacement.
//!
//! ## Two of them, and the difference is where the repeats land
//!
//! [`apply`] is the plain one: one line, one channel, no knowledge that there
//! is another side. Run over both through [`Stereo::each`] it gives a centred
//! source two identical echoes, which places that sound in **time** and never
//! in width — correct, and invisible.
//!
//! [`ping_pong`] is the stereo one, and it cannot be reached that way: the
//! left line's output is what feeds the right line, so the two sides are the
//! same delay rather than two of it. The input enters on one side only, which
//! is what makes the first repeat land left, the second right, and the tail
//! walk across the field. That cross-feed is the whole of the difference —
//! same `time`, same `feedback`, same tail length.
//!
//! The send is the **mono fold-down** of the input, for the reason
//! [`reverb`](super::reverb) gives for its own: a ping-pong is one device
//! everything goes into, and the width appears on the way out rather than
//! being carried in. A signal already dead centre therefore feeds it at
//! exactly its own level.

use crate::stereo::Stereo;

/// Apply a feedback delay to `buf` in place.
///
/// `time` is the echo spacing in seconds, `feedback` how much of each echo feeds
/// the next (clamped below 1 so the tail always dies), `mix` the wet/dry blend.
pub(crate) fn apply(buf: &mut [f32], time: f32, feedback: f32, mix: f32, rate: f32) {
    let Some(len) = line_length(time, buf.len(), rate) else {
        return;
    };
    let feedback = feedback.clamp(0.0, 0.95);
    let mix = mix.clamp(0.0, 1.0);
    let mut line = vec![0.0f32; len];
    let mut read = 0usize;
    for s in buf.iter_mut() {
        let echo = line[read];
        line[read] = *s + echo * feedback;
        read = (read + 1) % len;
        *s = *s * (1.0 - mix) + echo * mix;
    }
}

/// Apply a ping-pong delay to `buf` in place: the first repeat on the left,
/// the next on the right, and so on down the tail.
///
/// Two lines, cross-fed. The mono send enters the left one, the left one's
/// output feeds the right one, and the right one's output feeds the left —
/// so an echo lands on alternate sides at exactly the spacing [`apply`] would
/// have put them on both, and decays by `feedback` per repeat exactly as it
/// would have there. Which is why [`tail_seconds`] is the same arithmetic: a
/// tail is the same length however the repeats are placed.
pub(crate) fn ping_pong(buf: &mut Stereo, time: f32, feedback: f32, mix: f32, rate: f32) {
    let Some(len) = line_length(time, buf.frames(), rate) else {
        return;
    };
    let feedback = feedback.clamp(0.0, 0.95);
    let mix = mix.clamp(0.0, 1.0);
    let mut left = vec![0.0f32; len];
    let mut right = vec![0.0f32; len];
    let mut read = 0usize;
    for i in 0..buf.frames() {
        let (dry_l, dry_r) = (buf.l[i], buf.r[i]);
        let (echo_l, echo_r) = (left[read], right[read]);
        left[read] = (dry_l + dry_r) * 0.5 + echo_r * feedback;
        right[read] = echo_l * feedback;
        read = (read + 1) % len;
        buf.l[i] = dry_l * (1.0 - mix) + echo_l * mix;
        buf.r[i] = dry_r * (1.0 - mix) + echo_r * mix;
    }
}

/// How many samples one echo is, or `None` for a delay there is no point
/// running.
///
/// A non-positive or non-finite `time` is not a delay at all, and an echo
/// further away than eight times the signal itself would never be heard —
/// skipping beats allocating a line nobody reads. Both answers are the same
/// for one line or two, which is why they are asked here rather than twice.
fn line_length(time: f32, frames: usize, rate: f32) -> Option<usize> {
    if !time.is_finite() || time <= 0.0 {
        return None;
    }
    let len = ((time * rate).round() as usize).max(1);
    (len < frames.max(1) * 8).then_some(len)
}

/// How long the echoes keep ringing after the dry signal stops, in seconds — what
/// the renderer pads the buffer by so the tail is not cut off mid-repeat.
pub(crate) fn tail_seconds(time: f32, feedback: f32) -> f32 {
    let time = time.clamp(0.0, 4.0);
    let feedback = feedback.clamp(0.0, 0.95);
    if feedback <= 0.0 {
        return time;
    }
    // Repeats until the echo is 60 dB down, i.e. feedback^n = 0.001.
    let repeats = (0.001f32.ln() / feedback.ln()).ceil().clamp(1.0, 32.0);
    (time * repeats).min(4.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A single full-scale impulse followed by silence.
    fn impulse(n: usize) -> Vec<f32> {
        let mut buf = vec![0.0; n];
        buf[0] = 1.0;
        buf
    }

    #[test]
    fn the_echo_lands_one_delay_time_later_and_decays() {
        let mut buf = impulse(44_100);
        apply(&mut buf, 0.1, 0.5, 1.0, 44_100.0);
        // Fully wet: the dry impulse is gone and echoes appear every 4410 samples.
        assert!(buf[0].abs() < 1e-6, "fully wet keeps no dry signal");
        assert!((buf[4410] - 1.0).abs() < 1e-6, "first echo at 100 ms");
        assert!((buf[8820] - 0.5).abs() < 1e-6, "second echo, halved");
        assert!((buf[13230] - 0.25).abs() < 1e-6, "third echo, halved again");
    }

    #[test]
    fn mix_blends_wet_against_dry() {
        let mut buf = impulse(22_050);
        apply(&mut buf, 0.1, 0.0, 0.25, 44_100.0);
        assert!((buf[0] - 0.75).abs() < 1e-6, "dry is scaled by 1 - mix");
        assert!((buf[4410] - 0.25).abs() < 1e-6, "echo is scaled by mix");
    }

    #[test]
    fn runaway_feedback_is_clamped_so_the_tail_always_dies() {
        let mut buf = impulse(44_100);
        apply(&mut buf, 0.01, 5.0, 1.0, 44_100.0);
        assert!(buf.iter().all(|s| s.abs() <= 1.0 + 1e-6), "no runaway");
        let last = buf[40_000..].iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(last < 0.2, "the tail decayed to {last}");
    }

    #[test]
    fn a_degenerate_time_is_a_no_op_rather_than_a_panic() {
        let original = impulse(1024);
        for time in [0.0, -1.0, f32::INFINITY, 1000.0] {
            let mut buf = original.clone();
            apply(&mut buf, time, 0.5, 0.5, 44_100.0);
            assert!(buf.iter().all(|s| s.is_finite()), "time {time}");
            assert_eq!(line_length(time, 1024, 44_100.0), None, "time {time}");
            // And the same answer reaches both, since they ask it once.
            let mut wide = Stereo::centred(original.clone());
            ping_pong(&mut wide, time, 0.5, 0.5, 44_100.0);
            assert_eq!(wide, Stereo::centred(original.clone()), "time {time}");
        }
        assert_eq!(line_length(0.1, 44_100, 44_100.0), Some(4410));
        // The far bound exactly, where an echo stops being one worth running:
        // a line eight times the buffer is skipped and one sample under it is
        // not. Read at a rate and a length that make the boundary a round
        // number — 1000 frames at 8 kHz put it at exactly one second.
        assert_eq!(line_length(1.0, 1000, 8000.0), None, "eight times over");
        assert_eq!(line_length(0.999_875, 1000, 8000.0), Some(7999));
    }

    /// The whole of what `ping_pong` is: the repeats walk across the field
    /// instead of staying where the dry signal was. One echo per side per two
    /// delay times, at exactly the spacing and exactly the decay the plain
    /// delay would have put on both.
    #[test]
    fn the_repeats_land_on_alternate_sides() {
        let mut buf = Stereo::centred(impulse(44_100));
        ping_pong(&mut buf, 0.1, 0.5, 1.0, 44_100.0);
        assert!(buf.l[0].abs() < 1e-6, "fully wet keeps no dry signal");
        assert!((buf.l[4410] - 1.0).abs() < 1e-6, "first repeat, left");
        assert_eq!(buf.r[4410], 0.0, "and nothing on the right yet");
        assert!((buf.r[8820] - 0.5).abs() < 1e-6, "second repeat, right");
        assert_eq!(buf.l[8820], 0.0);
        assert!(
            (buf.l[13230] - 0.25).abs() < 1e-6,
            "third, back on the left"
        );
        assert_eq!(buf.r[13230], 0.0);
    }

    /// The send is the mono fold-down, so a centred signal feeds it at exactly
    /// its own level and `mix` blends against the dry side it was on.
    ///
    /// **Both sides**, and the right one is not a formality. It is the only
    /// channel whose dry signal never becomes an echo of its own — every
    /// repeat it carries arrived from the left line — so a blend that was
    /// wrong there would leave the left one reading correctly throughout, and
    /// the fully-wet test above cannot see it because at `mix` of one the dry
    /// term is multiplied by zero.
    #[test]
    fn a_centred_signal_feeds_it_at_its_own_level() {
        let mut buf = Stereo::centred(impulse(22_050));
        ping_pong(&mut buf, 0.1, 0.0, 0.25, 44_100.0);
        assert!((buf.l[0] - 0.75).abs() < 1e-6, "dry is scaled by 1 - mix");
        assert!((buf.l[4410] - 0.25).abs() < 1e-6, "and the repeat by mix");

        // The right side under the same rule, at a feedback that gives it a
        // repeat to blend: the second echo is 0.5 before the blend, so it
        // arrives at a quarter of that, over a dry impulse at three quarters.
        let mut both = Stereo::centred(impulse(22_050));
        ping_pong(&mut both, 0.1, 0.5, 0.25, 44_100.0);
        assert!((both.r[0] - 0.75).abs() < 1e-6, "right dry: {}", both.r[0]);
        assert!(
            (both.r[8820] - 0.125).abs() < 1e-6,
            "right repeat: {}",
            both.r[8820]
        );
        // Hard left in, and half of it reaches the line — the same fold-down
        // a reverb send does, so a wide input is not louder than a centred one.
        let mut side = Stereo {
            l: impulse(22_050),
            r: vec![0.0; 22_050],
        };
        ping_pong(&mut side, 0.1, 0.0, 1.0, 44_100.0);
        assert!((side.l[4410] - 0.5).abs() < 1e-6);
    }

    /// What it does to the width, which is the reason to reach for it and also
    /// the one way it can be got wrong.
    ///
    /// On a **hit** the two sides carry echoes at different instants, so there
    /// is nothing common between them and nothing cancelling either: dead
    /// zero, which is as wide as a signal goes without becoming a defect.
    ///
    /// On a **sustained tone** it is a defect waiting for the wrong number. A
    /// tone is still itself a delay later, so the odd repeats and the even
    /// ones are the same waveform at two phases — and where the delay is close
    /// to an odd number of half-periods, those phases are opposite and the two
    /// sides cancel in mono. It reaches −1.00 here, which is further than the
    /// [`chorus`](super::chorus) can go. The fix is the chorus's fix, and
    /// `docs/recipes.md` says so where it explains the figure: a `mix` that
    /// leaves the dry signal in the middle puts the correlation back above
    /// zero.
    #[test]
    fn a_hit_comes_back_wide_and_a_tone_can_come_back_cancelling() {
        let mut hit = Stereo::centred(impulse(44_100));
        ping_pong(&mut hit, 0.1, 0.5, 1.0, 44_100.0);
        assert_eq!(correlation(&hit), Some(0.0), "disjoint in time, so wide");

        // 100 Hz against a 5 ms delay: half a period exactly, so every repeat
        // is the inverse of the one before it.
        let tone: Vec<f32> = (0..44_100)
            .map(|i| (std::f32::consts::TAU * 100.0 * i as f32 / 44_100.0).sin())
            .collect();
        let mut wet = Stereo::centred(tone.clone());
        ping_pong(&mut wet, 0.005, 0.5, 1.0, 44_100.0);
        let cancelling = correlation(&wet).expect("a tone has two live sides");
        assert!(cancelling < -0.9, "a fully wet tone reads {cancelling}");

        let mut damp = Stereo::centred(tone);
        ping_pong(&mut damp, 0.005, 0.5, 0.3, 44_100.0);
        let kept = correlation(&damp).expect("a tone has two live sides");
        assert!(kept > 0.0, "the dry signal puts it back: {kept}");
    }

    /// How much of the signal is common to both channels — the same figure a
    /// bake report prints, read through the same meter.
    fn correlation(buf: &Stereo) -> Option<f64> {
        let mut meter = crate::level::Meter::new(2);
        meter.feed(&buf.interleaved());
        meter.correlation()
    }

    #[test]
    fn tail_length_covers_the_audible_repeats() {
        assert!(
            (tail_seconds(0.25, 0.0) - 0.25).abs() < 1e-6,
            "one echo only"
        );
        assert!(tail_seconds(0.25, 0.5) >= 2.0, "10 repeats at 60 dB down");
        assert!(tail_seconds(3.0, 0.9) <= 4.0, "but bounded");
    }
}
