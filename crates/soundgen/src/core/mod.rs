//! core — the note renderer: patch + note → `f32` buffer.
//!
//! This is the layer's contract made executable: *(pitch, velocity, duration) in →
//! buffer out*. The signal path is fixed, which is the whole point of a structured
//! patch — the recipe picks what fills each stage, never how they connect:
//!
//! ```text
//!   source ─► filter ─► amp envelope ─► fx chain
//!      ▲         ▲            ▲
//!      └─────── LFO ──────────┘   (one target: pitch | cutoff | amp)
//! ```
//!
//! Everything is `f32` in `−1..=1` end to end; the only quantisation is the WAV
//! encoder's. The rendered buffer is **longer than the note**: the amp
//! envelope's release rings out after the gate closes, and an fx chain adds its
//! own tail (an echo cut off mid-repeat is a click, not an echo).
//!
//! Module layout: [`osc`] (band-limited oscillator stack), [`karplus`] (plucked
//! string), [`fm`] (2-op FM), [`noise`] (the one seeded RNG), [`source`] (which of
//! those runs), [`mod@env`] (ADSR), [`filter`] (state-variable filter).

pub mod env;
pub mod filter;
pub mod fm;
pub mod karplus;
pub mod noise;
pub mod osc;
pub mod source;

use std::f32::consts::TAU;

use super::error::SynthError;
use super::fx;
use super::note::{NoteOpts, midi_to_freq};
use super::patch::{Filter, Lfo, LfoTarget, Patch};

/// The one rate everything here renders at. 44.1 kHz is CD rate: the full
/// audible band.
///
/// Fixed rather than chosen per call, and deliberately so. A bake is addressed
/// by the hash of the recipe that made it, so it must not vary with what some
/// later render happens to ask for — and `scorsese-render` resamples every
/// audio source on the way into the mix anyway. The reverb's delay lines are
/// tuned against this number; changing it would detune them silently.
pub const SAMPLE_RATE: u32 = 44_100;

/// [`SAMPLE_RATE`] as the float the DSP works in.
pub const RATE: f32 = SAMPLE_RATE as f32;

/// Longest buffer one note may render to. A guard, not a musical limit: a typo in
/// `duration` should fail loudly rather than allocate for an hour of audio.
const MAX_SECONDS: f32 = 60.0;

/// Renders one note of `patch` at MIDI pitch `midi` under `opts`, returning the
/// raw `f32` buffer — **pre-limiter**, so a caller summing several notes can
/// limit the sum instead of each part of it.
pub fn render_note(patch: &Patch, midi: f32, opts: &NoteOpts) -> Result<Vec<f32>, SynthError> {
    patch.validate()?;
    let gate = gate_length(opts.duration)?;
    let n = sample_count(gate + patch.amp.r.max(0.0) + fx::tail_seconds(&patch.fx));

    let freqs = pitch_track(patch.lfo, midi_to_freq(midi), n);
    let mut buf = vec![0.0; n];
    source::render(&patch.source, &freqs, opts.seed, &mut buf, RATE);
    if let Some(f) = patch.filter {
        filter::apply(&mut buf, &f, &cutoff_track(&f, patch.lfo, gate, n), RATE);
    }
    apply_amp(&mut buf, patch, gate, opts.velocity);
    fx::apply_chain(&mut buf, &patch.fx, RATE);
    Ok(buf)
}

/// Validates the requested note length, rejecting the values that would render
/// nothing at all.
fn gate_length(duration: f32) -> Result<f32, SynthError> {
    if !duration.is_finite() || duration <= 0.0 {
        return Err(SynthError::BadDuration { duration });
    }
    Ok(duration)
}

/// Renders one note and limits it — what a single baked one-shot needs, so it
/// cannot clip its own file.
///
/// Kept apart from [`render_note`] because the song mixer wants the unlimited
/// form: limiting every note before summing them would squash each one's
/// dynamics and then squash the sum again.
pub fn render_limited(patch: &Patch, midi: f32, opts: &NoteOpts) -> Result<Vec<f32>, SynthError> {
    let mut buf = render_note(patch, midi, opts)?;
    fx::limiter::apply(&mut buf, RATE);
    Ok(buf)
}

/// Samples needed for `seconds` of audio, at least one and at most [`MAX_SECONDS`].
fn sample_count(seconds: f32) -> usize {
    ((seconds.clamp(0.0, MAX_SECONDS) * RATE).ceil() as usize).max(1)
}

/// The LFO's raw `−1..1` sine value at sample `i`.
fn lfo_wave(lfo: &Lfo, i: usize) -> f32 {
    (TAU * lfo.rate.max(0.0) * i as f32 / RATE).sin()
}

/// The per-sample frequency track: the played pitch, bent by an LFO aimed at
/// `pitch` (`depth` semitones either way).
fn pitch_track(lfo: Option<Lfo>, base: f32, n: usize) -> Vec<f32> {
    match lfo {
        Some(l) if l.target == LfoTarget::Pitch => (0..n)
            .map(|i| base * (l.depth * lfo_wave(&l, i) / 12.0).exp2())
            .collect(),
        _ => vec![base; n],
    }
}

/// The per-sample cutoff track: the base cutoff, swept by the filter envelope
/// (`env_amount` Hz at full level) and wobbled by an LFO aimed at `cutoff`
/// (`depth` octaves either way).
fn cutoff_track(f: &Filter, lfo: Option<Lfo>, gate: f32, n: usize) -> Vec<f32> {
    let envelope = env::track(&f.adsr, gate, n, RATE);
    (0..n)
        .map(|i| {
            let swept = f.cutoff + f.env_amount * envelope[i];
            match lfo {
                Some(l) if l.target == LfoTarget::Cutoff => {
                    swept * (l.depth * lfo_wave(&l, i)).exp2()
                }
                _ => swept,
            }
        })
        .collect()
}

/// Apply velocity, the amp envelope and any tremolo — the stage that turns a
/// continuous tone into a note.
fn apply_amp(buf: &mut [f32], patch: &Patch, gate: f32, velocity: f32) {
    let envelope = env::track(&patch.amp, gate, buf.len(), RATE);
    let velocity = velocity.clamp(0.0, 1.0);
    for (i, s) in buf.iter_mut().enumerate() {
        *s *= velocity * envelope[i] * tremolo(patch.lfo, i);
    }
}

/// The tremolo gain at sample `i`: an LFO aimed at `amp` dips the level by `depth`
/// (so `depth = 1` dips all the way to silence).
fn tremolo(lfo: Option<Lfo>, i: usize) -> f32 {
    match lfo {
        Some(l) if l.target == LfoTarget::Amp => {
            1.0 - l.depth.clamp(0.0, 1.0) * 0.5 * (1.0 - lfo_wave(&l, i))
        }
        _ => 1.0,
    }
}
