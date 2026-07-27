//! Shared fixtures. Each test target uses a different slice of this, so unused
//! items here are expected rather than dead.
#![allow(dead_code)]

pub(crate) mod songs;

use scorsese_soundgen::patch::{Adsr, Osc, Patch, Source, Wave};
use scorsese_soundgen::{NoteOpts, SynthError};

/// A plain saw through a fully-sustaining envelope — the control case for
/// anything asking what a stage did to the signal.
pub(crate) fn saw_patch() -> Patch {
    Patch {
        source: Source::OscStack {
            oscs: vec![osc(Wave::Saw, 0.0, 0)],
        },
        amp: Adsr {
            a: 0.0,
            d: 0.0,
            s: 1.0,
            r: 0.0,
        },
        filter: None,
        lfo: None,
        fx: vec![],
    }
}

/// The smallest legal patch around `source` — every optional stage absent.
pub(crate) fn minimal(source: Source) -> Patch {
    Patch {
        source,
        amp: Adsr::default(),
        filter: None,
        lfo: None,
        fx: vec![],
    }
}

/// One oscillator, spelled out.
pub(crate) fn osc(wave: Wave, detune_cents: f32, octave: i32) -> Osc {
    Osc {
        wave,
        detune_cents,
        gain: 1.0,
        octave,
    }
}

/// An ADSR, positionally — the four numbers read better than four fields when
/// a test is about the shape rather than the names.
pub(crate) fn adsr(a: f32, d: f32, s: f32, r: f32) -> Adsr {
    Adsr { a, d, s, r }
}

/// A note of `duration` seconds, struck at full velocity, seed zero.
pub(crate) fn opts(duration: f32) -> NoteOpts {
    NoteOpts {
        duration,
        velocity: 1.0,
        seed: 0,
    }
}

/// The loudest sample in a buffer, as a magnitude.
pub(crate) fn peak(buf: &[f32]) -> f32 {
    buf.iter()
        .fold(0.0f32, |most, sample| most.max(sample.abs()))
}

/// How many times the signal crosses zero going upward — for a simple waveform
/// this *is* its frequency, which is how a pitch assertion avoids needing an
/// FFT.
pub(crate) fn rising_crossings(buf: &[f32]) -> usize {
    buf.windows(2)
        .filter(|pair| pair[0] <= 0.0 && pair[1] > 0.0)
        .count()
}

/// High-frequency content: the energy of the first difference relative to the
/// signal's own energy.
///
/// Amplitude-invariant — a quieter signal is not a duller one — which is what
/// makes it a fair brightness measure across a filter.
pub(crate) fn brightness(buf: &[f32]) -> f32 {
    let signal: f32 = buf.iter().map(|sample| sample * sample).sum();
    let edges: f32 = buf.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum();
    if signal > 0.0 { edges / signal } else { 0.0 }
}

/// Renders a note, failing the test with the synth's own words if it will not.
pub(crate) fn render(patch: &Patch, midi: f32, opts: &NoteOpts) -> Vec<f32> {
    scorsese_soundgen::render_note(patch, midi, opts).expect("the patch renders")
}

/// The error a patch is refused with — for the cases where *which* refusal
/// matters, not merely that there was one.
pub(crate) fn refusal(patch: &Patch, midi: f32, opts: &NoteOpts) -> SynthError {
    scorsese_soundgen::render_note(patch, midi, opts).expect_err("the patch is refused")
}
