//! the one source of randomness in a bake.
//!
//! Every stochastic element in the synth — the `noise` source, the Karplus-Strong
//! excitation — draws from here, and here draws from the **seeded integer hash**
//! `procgen` established ([`crate::hash`]). No `rand`, no wall clock: the
//! sample at index `i` is a pure function of `(i, channel, seed)`, which is what
//! makes "same patch + note + seed ⇒ byte-identical WAV" true rather than hoped for.

use crate::hash::unit2;
use crate::stereo::Stereo;

/// The hash channel the left side of a noise source draws on.
///
/// Zero, which is what a mono bake always drew from — so the left channel of a
/// noise source is the signal it has always been, and only the right one is
/// new.
const LEFT_CHANNEL: u64 = 0;
/// The hash channel the right side draws on. Its own, so the two sides are
/// **uncorrelated**, which is the entire point: two independent noise draws
/// are as wide as a signal can be, and they cost nothing.
const RIGHT_CHANNEL: u64 = 0x4e32; // "N2"

/// One white-noise sample in `−1..1` for position `i` on `channel`, under `seed`.
/// `channel` keeps two consumers (say the excitation and a noise layer) from
/// drawing the identical sequence.
#[inline]
pub(crate) fn white(i: usize, channel: u64, seed: u64) -> f32 {
    unit2(i as i64, 0, channel, seed) * 2.0 - 1.0
}

/// Fill `out` with white noise — the `noise` source, unshaped (the filter and amp
/// envelope downstream are what turn it into a gunshot or a footstep).
///
/// **The one source that is stereo at the source.** Every other generator here
/// makes a single waveform that reaches both channels identically, and width
/// is added downstream by the mix; noise is the exception because a second
/// independent draw is free and is *more* faithful to what noise is than a
/// copy of the first would be. A hiss identical in both ears is a point in
/// space; two draws are a wall of it.
pub(crate) fn fill(out: &mut Stereo, seed: u64) {
    for (i, s) in out.l.iter_mut().enumerate() {
        *s = white(i, LEFT_CHANNEL, seed);
    }
    for (i, s) in out.r.iter_mut().enumerate() {
        *s = white(i, RIGHT_CHANNEL, seed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled(n: usize, seed: u64) -> Stereo {
        let mut buf = Stereo::silence(n);
        fill(&mut buf, seed);
        buf
    }

    #[test]
    fn noise_is_bounded_and_centred() {
        let buf = filled(8192, 7);
        for channel in [&buf.l, &buf.r] {
            assert!(channel.iter().all(|s| (-1.0..=1.0).contains(s)));
            let mean: f32 = channel.iter().sum::<f32>() / channel.len() as f32;
            assert!(mean.abs() < 0.05, "mean {mean} is not centred on silence");
        }
    }

    #[test]
    fn the_same_seed_replays_the_same_noise() {
        assert_eq!(filled(256, 42), filled(256, 42));
    }

    #[test]
    fn a_different_seed_or_channel_decorrelates() {
        assert_ne!(filled(256, 42), filled(256, 43));
        assert_ne!(white(9, 0, 1), white(9, 1, 1));
    }

    /// The width the source is here for: the two sides are independent draws,
    /// not one draw copied, so the correlation between them is nothing like
    /// the 1.0 a duplicated channel would give.
    #[test]
    fn the_two_sides_are_independent_draws() {
        let buf = filled(8192, 11);
        assert_ne!(buf.l, buf.r);
        let n = buf.l.len() as f32;
        let dot: f32 = buf.l.iter().zip(&buf.r).map(|(l, r)| l * r).sum();
        let power = |c: &[f32]| c.iter().map(|s| s * s).sum::<f32>() / n;
        let correlation = (dot / n) / (power(&buf.l) * power(&buf.r)).sqrt();
        assert!(correlation.abs() < 0.05, "correlated at {correlation}");
    }
}
