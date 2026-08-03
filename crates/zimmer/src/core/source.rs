//! the source stage: which generator makes the tone
//!
//! One `match` over [`Source`], the head of the fixed signal path. Each arm hands
//! off to the module that owns that algorithm; nothing here knows any DSP. Sources
//! that can follow a moving pitch take the per-sample frequency track; Karplus-Strong
//! takes only the starting frequency, because its delay-line length is fixed once the
//! string is plucked.

use super::{fm, karplus, noise, osc};
use crate::patch::Source;

/// Render `source` into `out` (which the caller sized and zeroed), following the
/// per-sample frequency track `freqs`.
///
/// `velocity` is how hard the note was struck, already clamped to `0..=1` by
/// the caller. Only FM reads it here — its `vel_index` is depth a hard strike
/// adds to the modulator, and depth is the one place a source's *own*
/// brightness is a number the patch already carries. Every other source takes
/// its velocity further down the path, at the filter and the amp envelope.
///
/// The sum is resolved here rather than inside [`fm`] so that module stays the
/// FM algorithm and nothing else: it is handed the index to use, not the
/// bookkeeping that arrived at one.
pub(crate) fn render(
    source: &Source,
    freqs: &[f32],
    seed: u64,
    velocity: f32,
    out: &mut [f32],
    rate: f32,
) {
    match source {
        Source::OscStack { oscs } => osc::render(oscs, freqs, out, rate),
        Source::Karplus {
            damping,
            brightness,
        } => {
            let freq = freqs.first().copied().unwrap_or(0.0);
            karplus::render(out, freq, *damping, *brightness, seed, rate);
        }
        Source::Noise => noise::fill(out, seed),
        Source::Fm2 {
            ratio,
            index,
            vel_index,
            mod_decay,
        } => {
            let depth = (index + vel_index * velocity).max(0.0);
            fm::render(out, freqs, *ratio, depth, *mod_decay, rate);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::{Osc, Wave};

    fn render_kind(source: &Source) -> Vec<f32> {
        let mut out = vec![0.0; 8192];
        render(source, &vec![220.0; 8192], 5, 1.0, &mut out, 44_100.0);
        out
    }

    #[test]
    fn every_source_kind_produces_audible_finite_samples() {
        let kinds = [
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
        ];
        for source in kinds {
            let buf = render_kind(&source);
            assert!(
                buf.iter().all(|s| s.is_finite()),
                "{source:?} went non-finite"
            );
            let peak = buf.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            assert!(peak > 0.2, "{source:?} is inaudible (peak {peak})");
            assert!(peak <= 1.0 + 1e-3, "{source:?} peaked at {peak}");
        }
    }

    #[test]
    fn an_empty_frequency_track_does_not_panic() {
        let mut out = vec![];
        for source in [
            Source::Noise,
            Source::Karplus {
                damping: 0.9,
                brightness: 0.5,
            },
        ] {
            render(&source, &[], 1, 1.0, &mut out, 44_100.0);
        }
        assert!(out.is_empty());
    }
}
