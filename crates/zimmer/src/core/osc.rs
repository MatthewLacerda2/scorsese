//! the oscillator stack.
//!
//! The head of a subtractive patch: up to four oscillators, each with its own wave,
//! detune, octave and gain, summed and normalized. Detuning two saws a few cents
//! apart is the entire trick behind a "fat" analog lead, so the stack is the one
//! place a patch gets width.
//!
//! Sine and triangle are generated naively — they are band-limited enough by their
//! own spectra. Saw and square are **polyBLEP** corrected: a naive ramp or step has
//! infinite harmonics, which fold back as audible inharmonic garbage above a few
//! hundred Hz. PolyBLEP subtracts a small polynomial around each discontinuity,
//! costs ~20 lines, and removes the worst of that aliasing.
//!
//! Each oscillator keeps its own phase in `0..1`, advanced per sample by
//! `frequency / sample_rate`, so a per-sample frequency track (an LFO vibrato) needs
//! no special handling.
//!
//! **Where that phase starts is drawn from the note's seed, per oscillator.**
//! Starting every one of them at zero instead is three problems wearing one
//! coat, and none of them announces itself. A detuned pair begins
//! locked, so the drift the detune exists for starts from the same instant
//! every time and the first tens of milliseconds, the part the ear identifies a
//! sound by, are a photocopy. Two hits of one patch are then literally the same
//! samples, which no struck instrument has ever managed. And notes stacked into
//! a chord reinforce and cancel at exactly the same points, which is a
//! comb-filtered chord rather than a chord.
//!
//! The draw costs nothing and takes nothing away: it is a pure function of
//! `(seed, oscillator index)` through [`crate::hash`], so one recipe still bakes
//! one file, and [`crate::song`] already hands each note in an arrangement its
//! own seed, so a repeat differs from the note it repeats without the document
//! saying a word.
//!
//! **There is deliberately no way to hard-sync it.** The case for one is a
//! percussive one-shot whose transient is its identity — a kick whose click
//! varies is worse, not better — and it does not survive contact with this
//! stack. Phase zero is not the quiet place a hard sync would want: a saw
//! starts at −1, a triangle and a square at their extremes, and only a sine
//! ever began at silence, so a drawn phase is on average *closer* to a clean
//! start than the locked one it replaces. A one-shot bake carries its own seed
//! and so is identical every time it is used regardless; the only place a
//! strike varies is inside a song, where the percussive sources — noise, and
//! the Karplus excitation — have redrawn per note since they were written. The
//! field to add, the day a patch genuinely wants a stated start, is a phase per
//! oscillator rather than a boolean, and it should arrive with that patch
//! rather than ahead of it.

use std::f32::consts::TAU;

use crate::hash::unit2;
use crate::patch::{Osc, Wave};

/// Hash channel the start phases draw on, so a stack never mirrors the noise a
/// `noise` source or a Karplus excitation draws from the same note seed.
const PHASE_CHANNEL: u64 = 0x4f53; // "OS"

/// Render the summed stack for `freqs` (one base frequency per output sample) into
/// `out`. The mix is normalized by the total gain, so adding oscillators thickens
/// the tone without making it louder.
///
/// `seed` is the note's, and decides only where each oscillator starts in its
/// cycle — see the module doc for why that is not zero.
pub(crate) fn render(oscs: &[Osc], freqs: &[f32], seed: u64, out: &mut [f32], sample_rate: f32) {
    let total_gain: f32 = oscs.iter().map(|o| o.gain.max(0.0)).sum();
    let norm = if total_gain > 0.0 {
        1.0 / total_gain
    } else {
        0.0
    };
    for (index, osc) in oscs.iter().enumerate() {
        let ratio = pitch_ratio(osc);
        let gain = osc.gain.max(0.0) * norm;
        let mut phase = start_phase(index, seed);
        for (s, base) in out.iter_mut().zip(freqs) {
            let dt = (base * ratio / sample_rate).clamp(0.0, 0.5);
            *s += gain * sample(osc.wave, phase, dt);
            phase = (phase + dt).fract();
        }
    }
}

/// Where oscillator `index` of a note seeded `seed` starts in its cycle, in
/// `0..1`.
///
/// Per oscillator rather than per stack: one draw shared across the stack would
/// move a detuned pair together, which is the locked attack again wearing a
/// different offset.
fn start_phase(index: usize, seed: u64) -> f32 {
    unit2(index as i64, 0, PHASE_CHANNEL, seed)
}

/// The frequency multiplier an oscillator's octave transpose and cent detune imply.
fn pitch_ratio(osc: &Osc) -> f32 {
    (osc.octave as f32 + osc.detune_cents / 1200.0).exp2()
}

/// One sample of `wave` at normalized `phase` (`0..1`), where `dt` is the phase
/// increment per sample (the polyBLEP correction needs it to size its window).
fn sample(wave: Wave, phase: f32, dt: f32) -> f32 {
    match wave {
        Wave::Sine => (TAU * phase).sin(),
        // A phase-shifted triangle: peaks at 0, troughs at half a cycle.
        Wave::Triangle => 4.0 * (phase - 0.5).abs() - 1.0,
        Wave::Saw => 2.0 * phase - 1.0 - poly_blep(phase, dt),
        Wave::Square => {
            let naive = if phase < 0.5 { 1.0 } else { -1.0 };
            naive + poly_blep(phase, dt) - poly_blep((phase + 0.5).fract(), dt)
        }
    }
}

/// The polyBLEP residual: a two-sample parabolic smoothing of the step at phase 0,
/// which is what turns an aliasing discontinuity into a band-limited one.
fn poly_blep(phase: f32, dt: f32) -> f32 {
    if dt <= 0.0 {
        return 0.0;
    }
    if phase < dt {
        let t = phase / dt;
        2.0 * t - t * t - 1.0
    } else if phase > 1.0 - dt {
        let t = (phase - 1.0) / dt;
        t * t + 2.0 * t + 1.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn osc(wave: Wave, gain: f32, octave: i32, detune_cents: f32) -> Osc {
        Osc {
            wave,
            detune_cents,
            gain,
            octave,
        }
    }

    fn render_one(wave: Wave, hz: f32, n: usize) -> Vec<f32> {
        let mut out = vec![0.0; n];
        render(
            &[osc(wave, 1.0, 0, 0.0)],
            &vec![hz; n],
            7,
            &mut out,
            44_100.0,
        );
        out
    }

    /// Rising zero-crossings in a buffer — one per cycle, wherever the cycle
    /// began.
    fn rising(buf: &[f32]) -> usize {
        buf.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count()
    }

    #[test]
    fn every_wave_stays_inside_unity() {
        for wave in [Wave::Sine, Wave::Triangle, Wave::Saw, Wave::Square] {
            let buf = render_one(wave, 220.0, 4410);
            let peak = buf.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            assert!(peak <= 1.0 + 1e-3, "{wave:?} peaked at {peak}");
            assert!(peak > 0.5, "{wave:?} is suspiciously quiet ({peak})");
        }
    }

    #[test]
    fn a_sine_completes_its_frequency_in_cycles() {
        // 100 Hz over one second: 100 zero-crossings going positive, give or
        // take the one at the edge — the buffer no longer begins at phase zero,
        // so whether the hundredth crossing lands inside it depends on where in
        // its first cycle the oscillator started.
        let count = rising(&render_one(Wave::Sine, 100.0, 44_100));
        assert!((99..=100).contains(&count), "counted {count} cycles");
    }

    #[test]
    fn poly_blep_only_touches_the_discontinuity() {
        assert_eq!(poly_blep(0.5, 0.01), 0.0, "mid-cycle is untouched");
        assert!(poly_blep(0.0, 0.01) < 0.0, "correction at the step");
        assert!(poly_blep(0.999, 0.01) > 0.0, "and just before it");
        assert_eq!(poly_blep(0.5, 0.0), 0.0, "a stopped oscillator needs none");
    }

    #[test]
    fn band_limiting_tames_a_high_saw() {
        // The naive ramp jumps a full 2.0 every cycle; the corrected one should not
        // swing that hard sample-to-sample at a high pitch (5 kHz, dt ≈ 0.11).
        let buf = render_one(Wave::Saw, 5000.0, 4410);
        let worst = buf
            .windows(2)
            .fold(0.0f32, |m, w| m.max((w[1] - w[0]).abs()));
        assert!(
            worst < 1.6,
            "saw still steps by {worst} — polyBLEP is not applied"
        );
    }

    #[test]
    fn octave_and_detune_shift_the_pitch() {
        // An octave-down oscillator crosses zero half as often.
        let n = 44_100;
        let mut base = vec![0.0; n];
        let mut down = vec![0.0; n];
        render(
            &[osc(Wave::Sine, 1.0, 0, 0.0)],
            &vec![200.0; n],
            3,
            &mut base,
            44_100.0,
        );
        render(
            &[osc(Wave::Sine, 1.0, -1, 0.0)],
            &vec![200.0; n],
            3,
            &mut down,
            44_100.0,
        );
        assert!((199..=200).contains(&rising(&base)));
        assert!((99..=100).contains(&rising(&down)));
        assert!((pitch_ratio(&osc(Wave::Saw, 1.0, 0, 1200.0)) - 2.0).abs() < 1e-5);
    }

    #[test]
    fn the_stack_is_normalized_by_total_gain() {
        // Gains are weights, not levels: doubling every one of them changes
        // nothing, and a second oscillator thickens the tone without making it
        // louder. Both halves are read off peaks rather than samples because
        // the two oscillators no longer start in step.
        let n = 4410;
        let freqs = vec![440.0; n];
        let loudest = |oscs: &[Osc]| {
            let mut out = vec![0.0; n];
            render(oscs, &freqs, 5, &mut out, 44_100.0);
            out.iter().fold(0.0f32, |m, s| m.max(s.abs()))
        };
        let one = loudest(&[osc(Wave::Sine, 1.0, 0, 0.0)]);
        assert!((one - 1.0).abs() < 0.05, "a lone sine peaks at {one}");
        assert_eq!(loudest(&[osc(Wave::Sine, 8.0, 0, 0.0)]), one);
        let both = loudest(&[osc(Wave::Sine, 1.0, 0, 0.0), osc(Wave::Sine, 1.0, 0, 0.0)]);
        assert!(both <= one + 1e-3, "two peaked at {both}, one at {one}");
    }

    /// The whole point of drawing the phase, and the promise it must not cost:
    /// two notes of one patch differ, and either of them replays exactly.
    #[test]
    fn a_second_strike_differs_but_each_one_replays() {
        let struck = |seed: u64| {
            let mut out = vec![0.0; 2048];
            render(
                &[osc(Wave::Saw, 1.0, 0, 0.0), osc(Wave::Saw, 1.0, 0, 7.0)],
                &vec![220.0; 2048],
                seed,
                &mut out,
                44_100.0,
            );
            out
        };
        assert_eq!(struck(4), struck(4), "same seed, same samples");
        assert_ne!(struck(4), struck(5), "a second strike is not the first");
    }

    #[test]
    fn a_detuned_pair_does_not_start_in_step() {
        assert_ne!(start_phase(0, 9), start_phase(1, 9));
        assert_ne!(start_phase(0, 9), start_phase(0, 10));
        for index in 0..4 {
            let phase = start_phase(index, 9);
            assert!((0.0..1.0).contains(&phase), "phase {phase} is not a phase");
        }
    }

    #[test]
    fn a_stack_with_no_gain_left_is_silence_not_a_divide_by_zero() {
        // `Patch::validate` refuses this stack, so it can only arrive by calling the
        // renderer directly — and then the normalization must not divide by zero.
        let n = 512;
        let mut out = vec![0.0; n];
        let oscs = [osc(Wave::Saw, 0.0, 0, 0.0), osc(Wave::Sine, -1.0, 0, 0.0)];
        render(&oscs, &vec![440.0; n], 1, &mut out, 44_100.0);
        assert!(out.iter().all(|s| *s == 0.0), "got {:?}", &out[..8]);
    }
}
