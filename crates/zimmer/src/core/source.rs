//! the source stage: which generator makes the tone
//!
//! One `match` over [`Source`], the head of the fixed signal path. Each arm hands
//! off to the module that owns that algorithm; nothing here knows any DSP. Sources
//! that can follow a moving pitch take the per-sample frequency track; Karplus-Strong
//! takes only the starting frequency, because its delay-line length is fixed once the
//! string is plucked.
//!
//! **This is where mono becomes stereo, and mostly it does not.** An
//! oscillator stack, a plucked string and an FM pair are each one waveform, so
//! each is rendered once and placed in both channels — identically, which is
//! what makes a centred part carry exactly the samples it always did. Width is
//! a decision the mix makes, downstream, and a source that invented its own
//! would take that decision away from it.
//!
//! [`noise`] is the exception and owns the reason for it.

use super::{additive, fm, karplus, noise, osc};
use crate::patch::Source;
use crate::stereo::Stereo;

/// Render `frames` samples of `source`, following the per-sample frequency
/// track `freqs`.
///
/// `velocity` is how hard the note was struck, already clamped to `0..=1` by
/// the caller — the *brightness* velocity, which a performance may scatter a
/// little either side of the one the fader got. Only FM reads it here — its
/// `vel_index` is depth a hard strike adds to the modulator, and depth is the
/// one place a source's *own* brightness is a number the patch already
/// carries. Every other source takes its velocity further down the path, at
/// the filter and the amp envelope.
///
/// `seed` is the note's. Four of the five sources draw on it: the noise
/// source is nothing but, the Karplus excitation is a burst of it, and an
/// oscillator stack and an additive series each start their voices somewhere
/// in their cycle rather than all of them at zero.
///
/// The sum is resolved here rather than inside [`fm`] so that module stays the
/// FM algorithm and nothing else: it is handed the index to use, not the
/// bookkeeping that arrived at one.
pub(crate) fn render(
    source: &Source,
    freqs: &[f32],
    seed: u64,
    velocity: f32,
    frames: usize,
    rate: f32,
) -> Stereo {
    if matches!(source, Source::Noise) {
        let mut out = Stereo::silence(frames);
        noise::fill(&mut out, seed);
        return out;
    }
    let mut mono = vec![0.0; frames];
    one_waveform(source, freqs, seed, velocity, &mut mono, rate);
    Stereo::centred(mono)
}

/// The single waveform every source but `noise` produces.
fn one_waveform(
    source: &Source,
    freqs: &[f32],
    seed: u64,
    velocity: f32,
    out: &mut [f32],
    rate: f32,
) {
    match source {
        Source::OscStack { oscs } => osc::render(oscs, freqs, seed, out, rate),
        Source::Karplus {
            damping,
            brightness,
        } => {
            let freq = freqs.first().copied().unwrap_or(0.0);
            karplus::render(out, freq, *damping, *brightness, seed, rate);
        }
        Source::Fm2 {
            ratio,
            index,
            vel_index,
            mod_decay,
        } => {
            let depth = (index + vel_index * velocity).max(0.0);
            fm::render(out, freqs, *ratio, depth, *mod_decay, rate);
        }
        Source::Additive { partials } => additive::render(partials, freqs, seed, out, rate),
        // [`render`] sends this one down its own path before narrowing the
        // signal to one channel: it is the only source that draws two.
        Source::Noise => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::{Osc, Partial, Wave};

    fn render_kind(source: &Source) -> Stereo {
        render(source, &vec![220.0; 8192], 5, 1.0, 8192, 44_100.0)
    }

    fn every_kind() -> [Source; 5] {
        [
            Source::OscStack {
                oscs: vec![Osc {
                    wave: Wave::Saw,
                    detune_cents: 0.0,
                    gain: 1.0,
                    octave: 0,
                }],
            },
            Source::Karplus {
                damping: 0.99,
                brightness: 0.5,
            },
            Source::Noise,
            Source::Fm2 {
                ratio: 2.0,
                index: 4.0,
                vel_index: 0.0,
                mod_decay: 0.3,
            },
            Source::Additive {
                partials: vec![
                    Partial {
                        ratio: 1.0,
                        gain: 1.0,
                        detune_cents: 0.0,
                        decay: 0.0,
                    },
                    Partial {
                        ratio: 2.0,
                        gain: 0.5,
                        detune_cents: 0.0,
                        decay: 0.0,
                    },
                ],
            },
        ]
    }

    #[test]
    fn every_source_kind_produces_audible_finite_samples() {
        for source in every_kind() {
            let stereo = render_kind(&source);
            for buf in [&stereo.l, &stereo.r] {
                assert!(
                    buf.iter().all(|s| s.is_finite()),
                    "{source:?} went non-finite"
                );
                let peak = buf.iter().fold(0.0f32, |m, s| m.max(s.abs()));
                assert!(peak > 0.2, "{source:?} is inaudible (peak {peak})");
                assert!(peak <= 1.0 + 1e-3, "{source:?} peaked at {peak}");
            }
        }
    }

    /// The rule the module doc states, checked one source at a time: a
    /// generator makes one waveform and both channels get it, and `noise` is
    /// the single exception.
    #[test]
    fn only_noise_arrives_already_wide() {
        for source in every_kind() {
            let stereo = render_kind(&source);
            if matches!(source, Source::Noise) {
                assert_ne!(stereo.l, stereo.r, "noise draws each side its own");
            } else {
                assert_eq!(stereo.l, stereo.r, "{source:?} is one waveform, centred");
            }
        }
    }

    #[test]
    fn an_empty_frequency_track_does_not_panic() {
        for source in [
            Source::Noise,
            Source::Karplus {
                damping: 0.9,
                brightness: 0.5,
            },
        ] {
            assert!(render(&source, &[], 1, 1.0, 0, 44_100.0).is_empty());
        }
    }
}
