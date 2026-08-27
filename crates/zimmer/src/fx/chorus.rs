//! ensemble — several detuned, delayed copies of one signal, spread wide.
//!
//! The standard answer to *this is one synthesiser playing one note*, and the
//! reason is mechanical rather than tasteful. Five violinists are not one
//! violinist five times louder: they are five copies that disagree slightly
//! about pitch and about when the note started, and **the disagreement is the
//! sound**. Nothing else in this crate can make one source sound like more
//! than one thing — a filter, an envelope and a waveshaper each change what
//! the single thing sounds like, and a reverb puts the single thing in a room.
//!
//! Each voice is a delay line of about [`BASE_SECONDS`], read at a position a
//! slow sine walks back and forth by up to [`SWEEP_SECONDS`]. A moving read
//! position is a resampling, so a voice is *pitch-shifted* while the sweep
//! moves — sharp on the way in, flat on the way out — which is the detune. The
//! delays themselves are short enough that the copies fuse into one thickened
//! event rather than being heard as echoes.
//!
//! ## Linear interpolation is not an optimisation to skip
//!
//! A modulated delay read at integer sample positions steps between whole
//! samples, and each step is a discontinuity: what should be a smooth glide in
//! pitch arrives as a staircase of them, audible as zipper noise riding on the
//! effect. Linear interpolation between the two samples either side of the
//! fractional read position is the **minimum** that makes this a chorus rather
//! than a broken one, and it is what is done here. A higher-order interpolator
//! would buy a little high-frequency accuracy; it is not the difference
//! between working and not.
//!
//! ## Deterministic, and stratified
//!
//! The per-voice phase offsets and rate deviations come from [`crate::hash`]
//! under a fixed seed, so the ensemble is a pure function of the effect's own
//! settings — never of a counter carried between calls, and never of where in
//! a buffer processing began. That is the crate's determinism contract, and a
//! free-running LFO would break it the moment the same chain ran on a bus
//! instead of on a note.
//!
//! The phases are **stratified** rather than drawn uniformly: voice `v` takes
//! its offset from inside the `v`-th slice of the cycle. A uniform draw over
//! four voices will sometimes put two of them within a few degrees of each
//! other, and two voices in phase are not two voices — they are one voice
//! 6 dB louder, which is the exact failure the effect exists to avoid.
//!
//! ## Where the width comes from
//!
//! The voices read a **mono sum** of the input and are then panned across the
//! field, the way [`super::reverb`] sends mono into a stereo room. So a
//! centred source comes back wide, which is the point: the modulated copies
//! sitting outside the dry signal is half of what a chorus is, and the half a
//! mono implementation cannot have.
//!
//! Each side's voice gains are normalised to sum to one, so the wet signal
//! arrives at the level of the dry it is blended against however many voices
//! there are — a voice count is a thickness control, not a fader.
//!
//! It is not a width control either, and that is worth stating because it
//! reads like one. The voices are placed evenly across the field with both
//! ends occupied, so **the width is carried by the outermost pair** and the
//! ones between them fill the field in rather than stretching it: two voices
//! panned hard is the widest thing this makes, and four is the thickest. More
//! copies is more disagreement in the middle, which is what a section is.
//!
//! ## A flanger is a separate variant, and is not this one
//!
//! The structures look alike — a flanger is also a modulated delay line — and
//! this effect deliberately **cannot** be turned into one. A flanger's delay
//! is an order of magnitude shorter, down where the comb notches land inside
//! the range the ear reads as tone colour, and it feeds its output back into
//! its own input, which is where the resonant jet-sweep comes from. Without
//! the feedback path it is not a flanger; with one, this variant would carry a
//! field that means nothing for the ensemble it is named after, and a `voices`
//! count that means nothing for the flanger.
//!
//! So if a recipe ever needs a flanger it gets its own `Fx` variant, with its
//! own delay range and its own feedback. That is the layer's standing rule —
//! an effect arrives when a real sound cannot be made without it — and nothing
//! has needed one yet.

use std::f32::consts::TAU;

use crate::hash::unit2;
use crate::stereo::{Stereo, pan_gains};

/// Where a voice's delay sits before the sweep moves it, in seconds.
///
/// The classic chorus centre. Much under 10 ms and the comb notches climb into
/// the range heard as tone colour rather than thickness — that is the flanger
/// the module doc refuses; much over 30 ms and the copies stop fusing into one
/// event and start being heard as a slapback.
const BASE_SECONDS: f32 = 0.015;

/// How far either side of [`BASE_SECONDS`] a voice sweeps at `depth` 1, in
/// seconds. Ten milliseconds over a cycle is a detune of a few cents at any
/// usable rate — audible as richness, not as being out of tune.
const SWEEP_SECONDS: f32 = 0.010;

/// The fewest voices an ensemble is made of. Two: one copy is a detuned double
/// of the source with nowhere to be but beside it, and the field has two sides
/// to fill.
const MIN_VOICES: usize = 2;

/// The most voices an ensemble is made of.
///
/// Four is already a section. Past it each added voice sits closer to one
/// already there — the ear stops counting copies somewhere around three — and
/// the cost is another interpolated line read per sample for a thickness
/// nobody can name. It is the argument [`crate::patch::MAX_OSCS`] makes about
/// a stack, applied to arithmetic that runs over every sample.
const MAX_VOICES: usize = 4;

/// The fastest the sweep may run, in Hz. Past this a chorus is a ring
/// modulator: the sidebands the modulation puts either side of every partial
/// stop reading as a detune and start being inharmonic tones of their own.
const MAX_RATE: f32 = 10.0;

/// How far apart the voices' sweep rates are pulled, as a fraction of `rate`.
///
/// One shared rate gives an ensemble that breathes in unison, which is a
/// chorus pedal rather than a section. A tenth is enough that the voices drift
/// in and out of agreement over a few seconds without any of them running at a
/// noticeably different speed from the one written.
const RATE_SPREAD: f32 = 0.1;

/// The seed the phase and rate spreads are drawn under.
///
/// Fixed, and deliberately not the note's or the song's seed: the spread is a
/// property of *the effect*, so a chorus is the same ensemble on every note of
/// a part. Re-rolling it per note would make each note a differently-arranged
/// section, which is not what a section is.
const SPREAD_SEED: u64 = 0x0063_686f_7275_73;

/// The hash channel a voice's phase offset is drawn on.
const PHASE: u64 = 0;

/// The hash channel a voice's rate deviation is drawn on, so it does not
/// correlate with that voice's phase.
const DEVIATION: u64 = 1;

/// One modulated tap: where its sweep is, how fast it moves, where it sits.
struct Voice {
    /// Sweep phase in radians, advanced once per sample.
    phase: f32,
    /// Radians per sample — this voice's own rate, spread off the written one.
    step: f32,
    /// Left and right gains, already normalised across the ensemble.
    gains: (f32, f32),
}

/// Blend a modulated ensemble of `buf` into `buf`, in place.
///
/// `lfo_rate` is the sweep in Hz, `depth` how far it moves in `0..=1`,
/// `voices` how many copies (clamped to [`MIN_VOICES`]..=[`MAX_VOICES`]),
/// `mix` the wet/dry blend, and `rate` the sample rate.
pub(crate) fn apply(
    buf: &mut Stereo,
    lfo_rate: f32,
    depth: f32,
    voices: usize,
    mix: f32,
    rate: f32,
) {
    let mix = clamp_unit(mix);
    if mix <= 0.0 || buf.is_empty() || !rate.is_finite() || rate <= 0.0 {
        return;
    }
    let depth = clamp_unit(depth);
    let lfo_rate = if lfo_rate.is_finite() {
        lfo_rate.clamp(0.0, MAX_RATE)
    } else {
        0.0
    };
    let mut ensemble = ensemble(voices.clamp(MIN_VOICES, MAX_VOICES), lfo_rate, rate);

    // Two longer than the deepest read, so the interpolator's second tap can
    // never reach the sample being written this iteration.
    let len = (tail_seconds(depth) * rate).ceil() as usize + 2;
    let mut line = vec![0.0f32; len];
    let centre = BASE_SECONDS * rate;
    let sweep = depth * SWEEP_SECONDS * rate;

    for i in 0..buf.frames() {
        let (dry_l, dry_r) = (buf.l[i], buf.r[i]);
        line[i % len] = (dry_l + dry_r) * 0.5;
        let (mut wet_l, mut wet_r) = (0.0, 0.0);
        for voice in &mut ensemble {
            let tap = read(&line, i, centre + sweep * voice.phase.sin());
            wet_l += tap * voice.gains.0;
            wet_r += tap * voice.gains.1;
            voice.phase = (voice.phase + voice.step) % TAU;
        }
        buf.l[i] = dry_l * (1.0 - mix) + wet_l * mix;
        buf.r[i] = dry_r * (1.0 - mix) + wet_r * mix;
    }
}

/// How long the ensemble rings on after the signal stops, in seconds: the
/// deepest a voice is ever read from, and nothing more.
///
/// A chorus has no feedback path, so a sample enters the line once and leaves
/// it once — unlike an echo or a room, where the tail is a decay that has to
/// be estimated. This one is exact, and it is a fortieth of a second.
pub(crate) fn tail_seconds(depth: f32) -> f32 {
    BASE_SECONDS + clamp_unit(depth) * SWEEP_SECONDS
}

/// The voices, with their phases, rates and normalised placements resolved.
fn ensemble(count: usize, lfo_rate: f32, rate: f32) -> Vec<Voice> {
    let slice = TAU / count as f32;
    let mut voices: Vec<Voice> = (0..count)
        .map(|index| {
            let (ordinal, cell) = (index as f32, index as i64);
            // Stratified: the v-th offset lands inside the v-th slice of the
            // cycle, so no two voices can draw their way into agreement.
            let phase = (ordinal + unit2(cell, 0, PHASE, SPREAD_SEED)) * slice;
            let deviation = unit2(cell, 0, DEVIATION, SPREAD_SEED) * 2.0 - 1.0;
            Voice {
                phase,
                step: TAU * lfo_rate * (1.0 + RATE_SPREAD * deviation) / rate,
                // Evenly across the field, both ends included: the outermost
                // pair is what the width is carried by.
                gains: pan_gains(-1.0 + 2.0 * ordinal / (count - 1) as f32),
            }
        })
        .collect();
    let left: f32 = voices.iter().map(|voice| voice.gains.0).sum();
    let right: f32 = voices.iter().map(|voice| voice.gains.1).sum();
    for voice in &mut voices {
        voice.gains = (voice.gains.0 / left, voice.gains.1 / right);
    }
    voices
}

/// The line read `delay` samples behind write position `write`, linearly
/// interpolated between the two samples either side of that position.
///
/// `near` is the whole-sample part and `far` is one sample further back, so
/// the fraction weights *backwards* in time — the direction the delay grows.
fn read(line: &[f32], write: usize, delay: f32) -> f32 {
    let len = line.len();
    let whole = delay.floor();
    let frac = delay - whole;
    let back = (whole as usize).clamp(1, len - 2);
    let near = (write + len - back) % len;
    let far = (near + len - 1) % len;
    line[near] * (1.0 - frac) + line[far] * frac
}

/// A `0..=1` control, with anything that is not a number read as zero.
fn clamp_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 44_100.0;

    /// A second of a full-scale sine, dead centre.
    fn tone(freq: f32) -> Stereo {
        Stereo::centred(
            (0..RATE as usize)
                .map(|i| (TAU * freq * i as f32 / RATE).sin())
                .collect(),
        )
    }

    /// How alike the two channels are, `1.0` being identical.
    fn correlation(buf: &Stereo) -> f32 {
        let dot: f32 = buf.l.iter().zip(&buf.r).map(|(l, r)| l * r).sum();
        let energy = |c: &[f32]| c.iter().map(|s| s * s).sum::<f32>().sqrt();
        dot / (energy(&buf.l) * energy(&buf.r)).max(f32::MIN_POSITIVE)
    }

    #[test]
    fn a_centred_source_comes_back_wide() {
        let dry = tone(440.0);
        assert!((correlation(&dry) - 1.0).abs() < 1e-4, "it went in centred");
        let mut wet = dry.clone();
        apply(&mut wet, 0.6, 0.8, 4, 1.0, RATE);
        assert_ne!(wet.l, wet.r, "the sides disagree");
        let widened = correlation(&wet);
        assert!(widened < 0.9, "the sides are still {widened} alike");
    }

    #[test]
    fn the_same_settings_render_the_same_samples() {
        // The determinism contract: no counter, no clock, no free-running
        // phase — two calls on two buffers are bit-identical.
        let mut once = tone(220.0);
        let mut again = tone(220.0);
        apply(&mut once, 1.3, 0.7, 3, 0.6, RATE);
        apply(&mut again, 1.3, 0.7, 3, 0.6, RATE);
        assert_eq!(once, again);
    }

    #[test]
    fn the_voices_never_share_a_phase_or_a_rate() {
        for count in MIN_VOICES..=MAX_VOICES {
            let voices = ensemble(count, 1.0, RATE);
            assert_eq!(voices.len(), count);
            for (a, b) in voices.iter().zip(voices.iter().skip(1)) {
                assert!(b.phase > a.phase, "stratified, so strictly ordered");
                assert!((b.phase - a.phase) > 0.1, "and never near-coincident");
                assert_ne!(a.step, b.step, "each voice sweeps at its own rate");
            }
            let left: f32 = voices.iter().map(|v| v.gains.0).sum();
            assert!((left - 1.0).abs() < 1e-5, "the side sums to unity: {left}");
        }
    }

    #[test]
    fn the_modulated_read_glides_rather_than_stepping() {
        // A ramp read back at a slowly growing delay: with integer reads the
        // output would repeat samples and then jump. Linear interpolation
        // makes every consecutive difference smaller than one whole step.
        let line: Vec<f32> = (0..64).map(|i| i as f32).collect();
        let taps: Vec<f32> = (0..40)
            .map(|i| read(&line, 63, 8.0 + i as f32 * 0.05))
            .collect();
        for (a, b) in taps.iter().zip(taps.iter().skip(1)) {
            let step = (b - a).abs();
            assert!(step > 0.0 && step < 0.5, "a staircase step of {step}");
        }
    }

    #[test]
    fn a_dry_mix_and_a_degenerate_setting_change_nothing() {
        let original = tone(330.0);
        let mut dry = original.clone();
        apply(&mut dry, 0.5, 0.5, 3, 0.0, RATE);
        assert_eq!(dry, original, "a dry mix is not an approximation");
        for (lfo_rate, depth, rate) in [
            (f32::NAN, 0.5, RATE),
            (0.5, f32::INFINITY, RATE),
            (0.5, 0.5, 0.0),
            (1e9, 2.0, RATE),
        ] {
            let mut buf = original.clone();
            apply(&mut buf, lfo_rate, depth, 3, 1.0, rate);
            assert!(buf.l.iter().chain(&buf.r).all(|s| s.is_finite()));
        }
    }

    #[test]
    fn the_tail_is_the_deepest_read_and_nothing_more() {
        assert!((tail_seconds(0.0) - BASE_SECONDS).abs() < 1e-9);
        assert!((tail_seconds(1.0) - (BASE_SECONDS + SWEEP_SECONDS)).abs() < 1e-9);
        assert_eq!(tail_seconds(9.0), tail_seconds(1.0), "clamped");
        assert_eq!(tail_seconds(f32::NAN), tail_seconds(0.0));
        assert!(tail_seconds(1.0) < 0.03, "a chorus is not a room");
    }
}
