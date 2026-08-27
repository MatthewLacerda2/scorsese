//! Freeverb.
//!
//! Jezar's **Freeverb** is the public-domain reverb everyone reaches for at this
//! scale: eight parallel damped comb filters (the dense early build-up) feeding four
//! series allpass filters (the diffusion that smears them into a tail). Around a
//! hundred lines, no tuning tables to invent, and it has sounded good in trackers and
//! game audio for twenty-five years — exactly the "standard, proven algorithm"
//! bar this layer is held to.
//!
//! The comb/allpass lengths are the original prime-ish tunings, chosen at 44.1 kHz
//! and rescaled if a bake ever runs at another rate. `size` sets the comb feedback
//! (room size), `damp` how fast the tail loses its highs, `mix` the wet/dry blend.
//!
//! ## Both halves of it
//!
//! Freeverb is a **stereo** algorithm and always was: two banks of the same
//! twelve filters, the right one's delay lines a fixed [`STEREO_SPREAD`]
//! longer than the left's. That offset is the whole trick — the same room, but
//! its reflections arrive at the two ears at times that never quite line up,
//! which is what a room actually does and what a copy of one channel into the
//! other can never be. This crate carried only the left bank while it was
//! mono; restoring the right one is restoring a constant.
//!
//! Both banks are fed the **mono fold-down** of the input, which is what the
//! original does with its two inputs and what a physical send is: one room,
//! everything in front of it going in, the width appearing on the way out
//! rather than being carried in. A signal already dead centre therefore feeds
//! it at exactly its own level, and its left channel comes back the tail this
//! reverb has always produced.
//!
//! Jezar's `width` control, which bleeds each wet channel into the other, is
//! not carried across. At its default it is a no-op, and every setting other
//! than the default narrows the image the spread above just created — a knob
//! whose whole range is *less of the effect* is not a knob.

use crate::stereo::Stereo;

/// The eight parallel comb-filter lengths, in samples at 44.1 kHz.
const COMB_LENGTHS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
/// The four series allpass lengths, in samples at 44.1 kHz.
const ALLPASS_LENGTHS: [usize; 4] = [556, 441, 341, 225];
/// How much longer every delay line in the right bank is, in samples at
/// 44.1 kHz.
///
/// Jezar's `stereospread`, unchanged. Half a millisecond: far too short to
/// hear as an echo, and comfortably long enough that the two banks' comb
/// resonances land on different frequencies, which is what makes the tail
/// arrive from a width rather than from a point.
const STEREO_SPREAD: usize = 23;
/// The rate the tunings above were chosen at.
///
/// Equal to [`crate::SAMPLE_RATE`] today, so the rescale below is a no-op and
/// the reverb is exactly Freeverb's original voicing. It is kept as a separate
/// number rather than collapsed into one, because the two mean different
/// things: the render rate is a delivery decision, and *these* are the lengths
/// that make this particular room sound like a room. Collapsing them would
/// make a later change to the render rate silently retune the reverb.
const TUNED_RATE: f32 = 44_100.0;
/// Input scaling, so eight summed combs stay in range.
const FIXED_GAIN: f32 = 0.015;
/// `size` maps to comb feedback as `size × SCALE + OFFSET`.
const ROOM_SCALE: f32 = 0.28;
const ROOM_OFFSET: f32 = 0.7;
/// `damp` maps to the comb's internal lowpass coefficient by this scale.
const DAMP_SCALE: f32 = 0.4;
/// The allpass feedback Freeverb fixes at 0.5.
const ALLPASS_FEEDBACK: f32 = 0.5;

/// One damped comb filter: a delay line with a one-pole lowpass in its feedback
/// path, which is what makes the tail darken as it decays.
struct Comb {
    line: Vec<f32>,
    index: usize,
    store: f32,
}

impl Comb {
    fn new(len: usize) -> Self {
        Self {
            line: vec![0.0; len.max(1)],
            index: 0,
            store: 0.0,
        }
    }

    fn step(&mut self, input: f32, feedback: f32, damp: f32) -> f32 {
        let output = self.line[self.index];
        self.store = output * (1.0 - damp) + self.store * damp;
        self.line[self.index] = input + self.store * feedback;
        self.index = (self.index + 1) % self.line.len();
        output
    }
}

/// One allpass filter: passes every frequency at equal level but scrambles their
/// phases, turning the combs' discrete echoes into a smooth tail.
struct Allpass {
    line: Vec<f32>,
    index: usize,
}

impl Allpass {
    fn new(len: usize) -> Self {
        Self {
            line: vec![0.0; len.max(1)],
            index: 0,
        }
    }

    fn step(&mut self, input: f32) -> f32 {
        let buffered = self.line[self.index];
        self.line[self.index] = input + buffered * ALLPASS_FEEDBACK;
        self.index = (self.index + 1) % self.line.len();
        buffered - input
    }
}

/// One side of the room: the eight combs and four allpasses one channel's wet
/// signal passes through.
struct Bank {
    combs: Vec<Comb>,
    allpasses: Vec<Allpass>,
}

impl Bank {
    /// The bank for one side, its delay lines `offset` samples longer than the
    /// tuning table says and rescaled by `scale` if the render rate ever moves
    /// off the one the tunings were chosen at.
    fn new(offset: usize, scale: f32) -> Self {
        let scaled = |len: usize| (((len + offset) as f32 * scale).round() as usize).max(1);
        Self {
            combs: COMB_LENGTHS.iter().map(|l| Comb::new(scaled(*l))).collect(),
            allpasses: ALLPASS_LENGTHS
                .iter()
                .map(|l| Allpass::new(scaled(*l)))
                .collect(),
        }
    }

    /// One sample of send in, one sample of tail out.
    fn step(&mut self, send: f32, feedback: f32, damp: f32) -> f32 {
        let mut wet: f32 = self
            .combs
            .iter_mut()
            .map(|c| c.step(send, feedback, damp))
            .sum();
        for ap in self.allpasses.iter_mut() {
            wet = ap.step(wet);
        }
        wet
    }
}

/// Apply Freeverb to `buf` in place.
pub(crate) fn apply(buf: &mut Stereo, size: f32, damp: f32, mix: f32, rate: f32) {
    let scale = if rate > 0.0 { rate / TUNED_RATE } else { 1.0 };
    let mut left = Bank::new(0, scale);
    let mut right = Bank::new(STEREO_SPREAD, scale);

    let feedback = size.clamp(0.0, 1.0) * ROOM_SCALE + ROOM_OFFSET;
    let damp = damp.clamp(0.0, 1.0) * DAMP_SCALE;
    let mix = mix.clamp(0.0, 1.0);
    for (l, r) in buf.l.iter_mut().zip(buf.r.iter_mut()) {
        // One send, taken before either side is overwritten — the room hears
        // the mix, not the half of it that happens to be on this channel.
        let send = (*l + *r) * 0.5 * FIXED_GAIN;
        let wet_l = left.step(send, feedback, damp);
        let wet_r = right.step(send, feedback, damp);
        *l = *l * (1.0 - mix) + wet_l * mix;
        *r = *r * (1.0 - mix) + wet_r * mix;
    }
}

/// How long the tail rings after the dry signal stops, in seconds — what the
/// renderer pads the buffer by so the reverb is not cut off.
pub(crate) fn tail_seconds(size: f32) -> f32 {
    0.5 + size.clamp(0.0, 1.0) * 2.5
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A full-scale impulse then silence, dead centre — the classic reverb
    /// probe.
    fn impulse(n: usize) -> Stereo {
        let mut mono = vec![0.0; n];
        mono[0] = 1.0;
        Stereo::centred(mono)
    }

    /// Total energy in a window, the measure a tail is judged by.
    fn energy(buf: &[f32]) -> f32 {
        buf.iter().map(|s| s.abs()).sum()
    }

    /// The reverb's answer to a centred impulse.
    fn rung(n: usize, size: f32, damp: f32, mix: f32) -> Stereo {
        let mut buf = impulse(n);
        apply(&mut buf, size, damp, mix, 44_100.0);
        buf
    }

    #[test]
    fn an_impulse_becomes_a_decaying_tail() {
        let buf = rung(88_200, 0.8, 0.5, 1.0);
        for channel in [&buf.l, &buf.r] {
            let early = energy(&channel[..22_050]);
            let late = energy(&channel[66_150..]);
            assert!(early > 0.5, "the reverb produced almost nothing ({early})");
            assert!(late < early, "the tail must decay: {early} → {late}");
            assert!(channel.iter().all(|s| s.abs() <= 1.0), "and never clip");
        }
    }

    /// What the stereo spread is for: one room, but the two sides of it are
    /// different signals rather than one signal twice. Both carry a tail — a
    /// bank that had silently failed to build would read as "different" too.
    #[test]
    fn the_two_sides_of_the_room_are_not_the_same_tail() {
        let buf = rung(44_100, 0.8, 0.5, 1.0);
        assert_ne!(buf.l, buf.r);
        let (left, right) = (energy(&buf.l[4410..]), energy(&buf.r[4410..]));
        assert!(left > 0.5 && right > 0.5, "{left} / {right}");
        // Neither side is favoured: the same twelve filters, offset, so the
        // tails carry comparable energy.
        assert!(
            (left - right).abs() < left.max(right) * 0.5,
            "one side is far louder: {left} / {right}"
        );
    }

    /// A hard-panned source still reaches both sides of the room, because the
    /// send is the mix rather than the channel. A reverb that only wet the
    /// channel it was given would put a panned instrument's tail in the same
    /// place as the instrument, which is not what a room does.
    #[test]
    fn a_one_sided_source_still_rings_on_both_sides() {
        let mut buf = Stereo::silence(44_100);
        buf.l[0] = 1.0;
        apply(&mut buf, 0.8, 0.5, 1.0, 44_100.0);
        assert!(energy(&buf.r[4410..]) > 0.2, "the right side stayed dry");
    }

    #[test]
    fn a_bigger_room_rings_longer() {
        let tail = |size: f32| energy(&rung(88_200, size, 0.5, 1.0).l[44_100..]);
        assert!(tail(1.0) > tail(0.2) * 2.0);
        assert!(tail_seconds(1.0) > tail_seconds(0.0));
    }

    #[test]
    fn damping_dulls_the_tail() {
        let roughness = |damp: f32| {
            rung(44_100, 0.8, damp, 1.0).l[22_050..]
                .windows(2)
                .map(|w| (w[1] - w[0]).abs())
                .sum::<f32>()
        };
        assert!(roughness(1.0) < roughness(0.0), "damping removes highs");
    }

    #[test]
    fn a_dry_mix_leaves_the_signal_alone() {
        assert_eq!(rung(4096, 0.7, 0.5, 0.0), impulse(4096));
    }

    #[test]
    fn silence_in_is_silence_out_and_stays_finite() {
        let mut buf = Stereo::silence(4096);
        apply(&mut buf, 0.9, 0.2, 1.0, 44_100.0);
        assert!(buf.l.iter().chain(&buf.r).all(|s| *s == 0.0));
        let mut buf = impulse(4096);
        apply(&mut buf, 2.0, -1.0, 3.0, 0.0);
        assert!(
            buf.l.iter().chain(&buf.r).all(|s| s.is_finite()),
            "clamped, not exploded"
        );
    }
}
