//! fx — the post-chain: delay, reverb, drive, EQ, compression, chorus, and the
//! mandatory limiter
//!
//! Six a recipe chooses, one it does not. [`delay`] and [`reverb`] place a dry
//! synthesized sound *somewhere* — the difference between a gunshot recorded in an
//! anechoic chamber and one fired in a corridor. [`saturate`] is the odd one out,
//! and the only nonlinearity in the crate: it does not move a sound, it changes
//! what is *in* one, which is the difference between a clean sum of waveforms and
//! something that sounds recorded. [`eq`] and [`compress`] are the two mixing
//! moves: one takes a *region* away and is the treatment for what
//! [`crate::level::bands`] already diagnoses, the other takes the *loud moments*
//! down and is what makes a part sit rather than merely be at a volume.
//! [`chorus`] is the odd one against all of those: it does not place a sound,
//! colour it or take anything away — it makes **several** of it, slightly
//! detuned and slightly late, which is the only thing in this crate that can
//! stop one source sounding like one instrument playing. All six are chosen per
//! chain and applied in list order. [`limiter`] is not a choice: every bake
//! passes through it, because a clipped WAV is a broken asset, not a stylistic
//! option.
//!
//! **[`compress`] and [`limiter`] are not the same device.** They share the shape
//! of their implementation — a gain track computed for every sample, then ramped
//! in both directions — and nothing else. One is chosen, audible and free to add
//! gain back; the other is unconditional, ideally inaudible, and is the promise
//! that a bake cannot clip. That is also why a song's chain runs *before* the
//! limiter and never after: makeup gain past the ceiling would withdraw the
//! promise quietly, and [`crate::song`]'s mixer states the rule.
//!
//! Distortion, a flanger and the rest wait until a real sound cannot be made
//! without them (the layer's standing rule) — and a flanger is not a corner of
//! [`chorus`], for the reason that module's own doc gives. Effects are added
//! here, not in the patch document's signal path, so the fixed
//! source → filter → amp contract stays fixed.
//!
//! ## Which of these knows it is in stereo
//!
//! [`reverb`] does, and it was the first that had to: Freeverb was always a
//! stereo algorithm, and the two sides of a room are not the same room heard
//! twice. It is where most of the width in a finished piece comes from.
//!
//! [`chorus`] is the other, and it is the one that is not merely aware of two
//! channels but *makes* them: its copies read a mono sum and are panned across
//! the field, so a centred source comes back wider than it went in. That is
//! half of what a chorus is, and the half a per-channel implementation could
//! not have — which is why it was built after the crate went stereo rather
//! than twice.
//!
//! [`delay`], [`saturate`] and [`eq`] run **per channel, independently**. A
//! waveshaper is memoryless, and a delay line's and a biquad's memory each
//! belong to the side they are on, so every one of them gets its own and none
//! needs to see the other. On a signal that is still centred all three come
//! back centred, which is correct — a slapback is not a width effect and
//! neither is a shelf. A ping-pong delay, which *is*, is deliberately not
//! here: it needs a field on the document to ask for, and a field is a
//! decision to take with a recipe in front of you.
//!
//! [`limiter`] and [`compress`] are the third case: one gain, both channels —
//! see the limiter's own doc for the argument, which the compressor inherits
//! whole. A compressor that moved one side and not the other would be a fader
//! pulling the image sideways every time it acted, which is worse than one
//! that acts too much.

pub(crate) mod chorus;
pub(crate) mod compress;
pub(crate) mod delay;
pub(crate) mod eq;
pub(crate) mod limiter;
pub(crate) mod reach;
pub(crate) mod reverb;
pub(crate) mod saturate;

pub(crate) use reach::{lookahead_seconds, tail_seconds};

use crate::patch::Fx;
#[cfg(test)]
use crate::patch::{EqBand, EqKind};
use crate::stereo::Stereo;

/// Where a sidechained compressor finds the part it is keyed from.
///
/// A trait rather than a map because the lookup belongs to the mixer — it is
/// the only place every track exists separately — and this module has never
/// heard of a track. Two chain locations have no tracks to name at all, and
/// say so by answering `None` to everything.
pub(crate) trait Keys {
    /// The named track's part as played, or `None` if there is no such part
    /// here to hand over.
    fn part(&self, track: &str) -> Option<&Stereo>;
}

/// The lookup for a chain sitting somewhere a track name means nothing: a
/// patch's, which runs per note, and the song's own, which runs on the sum.
/// Both refuse a `sidechain` in validation, so this is what a chain that
/// passed that check needs and no more.
struct NoKeys;

impl Keys for NoKeys {
    fn part(&self, _track: &str) -> Option<&Stereo> {
        None
    }
}

/// Apply the chain to `buf` in place, in list order.
///
/// For the two chains that have no tracks to name — a patch's and the song's.
/// A compressor on one of those reads its level from the signal it is
/// changing, which is what a compressor with no `sidechain` does anyway.
pub(crate) fn apply_chain(buf: &mut Stereo, chain: &[Fx], rate: f32) {
    apply_chain_keyed(buf, chain, rate, &NoKeys);
}

/// [`apply_chain`], with somewhere for a sidechained compressor to find the
/// part it listens to — what a **track's** chain gets, and only a track's.
pub(crate) fn apply_chain_keyed(buf: &mut Stereo, chain: &[Fx], rate: f32, keys: &dyn Keys) {
    for fx in chain {
        match fx {
            Fx::Delay {
                time,
                feedback,
                mix,
            } => buf.each(|channel| delay::apply(channel, *time, *feedback, *mix, rate)),
            Fx::Reverb { size, damp, mix } => reverb::apply(buf, *size, *damp, *mix, rate),
            Fx::Saturate { drive, mix } => {
                buf.each(|channel| saturate::apply(channel, *drive, *mix));
            }
            Fx::Eq { bands } => buf.each(|channel| eq::apply(channel, bands, rate)),
            Fx::Compress {
                threshold,
                ratio,
                attack,
                release,
                makeup,
                mix,
                sidechain,
            } => {
                let key = sidechain.as_deref().and_then(|track| keys.part(track));
                compress::Compressor::new(*threshold, *ratio, *attack, *release, *makeup, *mix)
                    .apply(buf, key, rate);
            }
            // Not through `each`: an ensemble is made of copies placed
            // *against* each other, so it needs both sides at once.
            Fx::Chorus {
                rate: sweep,
                depth,
                voices,
                mix,
            } => chorus::apply(buf, *sweep, *depth, *voices, *mix, rate),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn impulse(n: usize) -> Stereo {
        let mut mono = vec![0.0; n];
        mono[0] = 1.0;
        Stereo::centred(mono)
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
        assert!(forward.l.iter().chain(&forward.r).all(|s| s.is_finite()));
    }

    /// Which effects widen a signal, checked rather than asserted in prose:
    /// the reverb and the chorus are the two that do, and the others hand back
    /// what they were given, in the place they were given it.
    #[test]
    fn only_the_room_and_the_ensemble_take_a_centred_signal_off_centre() {
        let mut room = impulse(22_050);
        apply_chain(
            &mut room,
            &[Fx::Reverb {
                size: 0.7,
                damp: 0.5,
                mix: 1.0,
            }],
            44_100.0,
        );
        assert_ne!(room.l, room.r, "a room has two sides");
        let mut section = impulse(22_050);
        apply_chain(
            &mut section,
            &[Fx::Chorus {
                rate: 0.8,
                depth: 0.7,
                voices: 4,
                mix: 1.0,
            }],
            44_100.0,
        );
        assert_ne!(section.l, section.r, "the copies are placed apart");
        let mut narrow = impulse(22_050);
        apply_chain(
            &mut narrow,
            &[
                Fx::Delay {
                    time: 0.05,
                    feedback: 0.4,
                    mix: 0.5,
                },
                Fx::Saturate {
                    drive: 4.0,
                    mix: 1.0,
                },
            ],
            44_100.0,
        );
        assert_eq!(narrow.l, narrow.r, "neither of these is a width effect");
    }

    #[test]
    fn a_waveshaper_reaches_the_signal_and_still_asks_for_no_tail() {
        let drive = [Fx::Saturate {
            drive: 4.0,
            mix: 1.0,
        }];
        assert_eq!(tail_seconds(&drive), 0.0, "there is no state to decay");
        let mut buf = Stereo::centred(vec![0.5]);
        buf.resize(512);
        apply_chain(&mut buf, &drive, 44_100.0);
        assert!(
            buf.l[0] > 0.9,
            "the sample was shaped, not passed: {}",
            buf.l[0]
        );
        assert!(
            buf.l[1..].iter().all(|s| *s == 0.0),
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
        let tone: Vec<f32> = (0..4410)
            .map(|i| (std::f32::consts::TAU * 250.0 * i as f32 / 44_100.0).sin())
            .collect();
        let mut buf = Stereo::centred(tone);
        // Read past the filter's own start-up transient, which is louder than
        // the settled response and is not what is being asserted about.
        let peak = |buf: &[f32]| buf[2205..].iter().fold(0.0f32, |m, s| m.max(s.abs()));
        let before = peak(&buf.l);
        apply_chain(&mut buf, &carve, 44_100.0);
        let after = peak(&buf.l);
        assert!(
            after < before * 0.5,
            "the band was cut: {before} to {after}"
        );
        // A biquad's state belongs to the channel it is on, so both sides get
        // their own and a centred tone comes back centred: an EQ is a mixing
        // move, never a width one.
        assert_eq!(buf.l, buf.r);
    }

    /// What a chain with no tracks to name does with a compressor that asks
    /// for one anyway: it reads its own level, exactly as a compressor with no
    /// `sidechain` does.
    ///
    /// Nothing valid arrives here — a patch chain and the song's own both
    /// refuse a `sidechain` in validation — but the fallback still has to be
    /// *the effect*, not *no effect*. A lookup that answered with an empty
    /// buffer would read as silence at every frame, and a compressor keyed
    /// from silence never acts, so the failure would be a chain that quietly
    /// stopped compressing rather than one that said something.
    #[test]
    fn a_chain_with_no_tracks_to_name_keys_a_compressor_from_its_own_signal() {
        let asked = [Fx::Compress {
            threshold: -20.0,
            ratio: 8.0,
            attack: 0.005,
            release: 0.05,
            makeup: 0.0,
            mix: 1.0,
            sidechain: Some("nowhere".to_owned()),
        }];
        let mut buf = Stereo::centred(vec![0.9; 4410]);
        apply_chain(&mut buf, &asked, 44_100.0);
        assert!(
            buf.l[2205] < 0.5,
            "it compressed nothing, so it read nothing: {}",
            buf.l[2205]
        );
        assert_eq!(buf.l, buf.r, "and one gain reached both sides");
    }
}
