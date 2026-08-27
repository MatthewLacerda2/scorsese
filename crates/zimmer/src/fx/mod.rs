//! fx — the post-chain: delay, reverb, drive, EQ, and the mandatory limiter
//!
//! Four a recipe chooses, one it does not. [`delay`] and [`reverb`] place a dry
//! synthesized sound *somewhere* — the difference between a gunshot recorded in an
//! anechoic chamber and one fired in a corridor. [`saturate`] is the odd one out,
//! and the only nonlinearity in the crate: it does not move a sound, it changes
//! what is *in* one, which is the difference between a clean sum of waveforms and
//! something that sounds recorded. [`eq`] takes things *away* — it is the only one
//! here that is a mixing move rather than a sound, and the treatment for what
//! [`crate::level::bands`] already diagnoses. All four are chosen per chain and
//! applied in list order. [`limiter`] is not a choice: every bake passes through
//! it, because a clipped WAV is a broken asset, not a stylistic option.
//!
//! Chorus and the rest wait until a real sound cannot be made without them (the
//! layer's standing rule). Effects are added here, not in the patch document's
//! signal path, so the fixed source → filter → amp contract stays fixed.

pub(crate) mod delay;
pub(crate) mod eq;
pub(crate) mod limiter;
pub(crate) mod reverb;
pub(crate) mod saturate;

use crate::patch::Fx;
#[cfg(test)]
use crate::patch::{EqBand, EqKind};

/// Longest fx tail a note is padded by, in seconds. A chain of long effects should
/// not turn a 200 ms blip into a half-minute file.
const MAX_TAIL: f32 = 6.0;

/// Apply the chain to `buf` in place, in list order.
pub(crate) fn apply_chain(buf: &mut [f32], chain: &[Fx], rate: f32) {
    for fx in chain {
        match fx {
            Fx::Delay {
                time,
                feedback,
                mix,
            } => delay::apply(buf, *time, *feedback, *mix, rate),
            Fx::Reverb { size, damp, mix } => reverb::apply(buf, *size, *damp, *mix, rate),
            Fx::Saturate { drive, mix } => saturate::apply(buf, *drive, *mix),
            Fx::Eq { bands } => eq::apply(buf, bands, rate),
        }
    }
}

/// How much extra time the chain needs to ring out after the note itself ends —
/// what the renderer pads the buffer by, so an echo or a reverb tail is never cut
/// off mid-repeat. A fully dry effect asks for nothing, and neither does a
/// waveshaper: [`saturate`] is memoryless, so there is no state left in it to
/// decay once the signal stops. Nor does [`eq`], which does have state but
/// neither delays nor repeats — its own module doc argues that one.
pub(crate) fn tail_seconds(chain: &[Fx]) -> f32 {
    let total: f32 = chain
        .iter()
        .map(|fx| match fx {
            Fx::Delay {
                time,
                feedback,
                mix,
            } if *mix > 0.0 => delay::tail_seconds(*time, *feedback),
            Fx::Reverb { size, mix, .. } if *mix > 0.0 => reverb::tail_seconds(*size),
            _ => 0.0,
        })
        .sum();
    total.min(MAX_TAIL)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn impulse(n: usize) -> Vec<f32> {
        let mut buf = vec![0.0; n];
        buf[0] = 1.0;
        buf
    }

    #[test]
    fn an_empty_chain_changes_nothing() {
        let mut buf = impulse(512);
        apply_chain(&mut buf, &[], 44_100.0);
        assert_eq!(buf, impulse(512));
        assert_eq!(tail_seconds(&[]), 0.0);
    }

    #[test]
    fn the_chain_applies_in_list_order() {
        // Delay-then-reverb smears the echoes; reverb-then-delay echoes the smear.
        // They must differ, which proves order is honoured rather than commuted.
        let chain = [
            Fx::Delay {
                time: 0.05,
                feedback: 0.4,
                mix: 0.5,
            },
            Fx::Reverb {
                size: 0.6,
                damp: 0.5,
                mix: 0.5,
            },
        ];
        let mut forward = impulse(22_050);
        let mut reversed = impulse(22_050);
        apply_chain(&mut forward, &chain, 44_100.0);
        apply_chain(
            &mut reversed,
            &[chain[1].clone(), chain[0].clone()],
            44_100.0,
        );
        assert_ne!(forward, reversed);
        assert!(forward.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn the_tail_covers_every_wet_effect_but_stays_bounded() {
        let wet = [
            Fx::Delay {
                time: 0.25,
                feedback: 0.5,
                mix: 0.5,
            },
            Fx::Reverb {
                size: 1.0,
                damp: 0.5,
                mix: 0.5,
            },
        ];
        assert!(tail_seconds(&wet[..1]) > 0.0);
        assert!(tail_seconds(&wet) > tail_seconds(&wet[..1]));
        assert!(tail_seconds(&wet) <= MAX_TAIL);
        let long = vec![
            Fx::Reverb {
                size: 1.0,
                damp: 0.5,
                mix: 1.0,
            };
            8
        ];
        assert_eq!(
            tail_seconds(&long),
            MAX_TAIL,
            "capped, however long the chain"
        );
    }

    #[test]
    fn a_waveshaper_reaches_the_signal_and_still_asks_for_no_tail() {
        let drive = [Fx::Saturate {
            drive: 4.0,
            mix: 1.0,
        }];
        assert_eq!(tail_seconds(&drive), 0.0, "there is no state to decay");
        let mut buf = vec![0.0; 512];
        buf[0] = 0.5;
        apply_chain(&mut buf, &drive, 44_100.0);
        assert!(
            buf[0] > 0.9,
            "the sample was shaped, not passed: {}",
            buf[0]
        );
        assert!(
            buf[1..].iter().all(|s| *s == 0.0),
            "and nothing after it moved, because there is no memory"
        );
    }

    #[test]
    fn an_eq_reaches_the_signal_and_still_asks_for_no_tail() {
        // A biquad has state, unlike the waveshaper above, so this is the
        // claim `eq`'s module doc makes rather than a definition: a filter
        // neither delays nor repeats, so a note carrying one does not grow.
        let carve = [Fx::Eq {
            bands: vec![EqBand {
                kind: EqKind::Peak,
                freq: Some(250.0),
                gain_db: -12.0,
                q: 2.0,
            }],
        }];
        assert_eq!(tail_seconds(&carve), 0.0, "no echo to be cut off");
        let mut buf: Vec<f32> = (0..4410)
            .map(|i| (std::f32::consts::TAU * 250.0 * i as f32 / 44_100.0).sin())
            .collect();
        // Read past the filter's own start-up transient, which is louder than
        // the settled response and is not what is being asserted about.
        let peak = |buf: &[f32]| buf[2205..].iter().fold(0.0f32, |m, s| m.max(s.abs()));
        let before = peak(&buf);
        apply_chain(&mut buf, &carve, 44_100.0);
        let after = peak(&buf);
        assert!(
            after < before * 0.5,
            "the band was cut: {before} to {after}"
        );
    }

    #[test]
    fn a_dry_effect_asks_for_no_tail() {
        let dry = [
            Fx::Delay {
                time: 1.0,
                feedback: 0.9,
                mix: 0.0,
            },
            Fx::Reverb {
                size: 1.0,
                damp: 0.5,
                mix: 0.0,
            },
        ];
        assert_eq!(tail_seconds(&dry), 0.0);
    }
}
