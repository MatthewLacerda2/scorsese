//! What the two envelope shapes do to the samples: a bend that changes how a
//! note dies, and a sweep that changes what pitch it starts on.
//!
//! Asked of the rendered buffer rather than of the envelope, which is the half
//! `core::env`'s own tests cannot answer — a shaping function that is perfect
//! and never reaches the signal is still a stage that does nothing.

use crate::common::{adsr, opts, peak, render, rising_crossings, saw_patch};
use scorsese_zimmer::patch::{Adsr, Lfo, LfoTarget, Patch, PitchEnv};

/// A tenth of a second at the render rate: the window every measurement below
/// is taken over, long enough to hold ten cycles of the lowest note used.
const WINDOW: usize = 4_410;

/// A saw whose level decays over a whole second, straight or bent. Nothing
/// else moves, so a difference in the samples is the curve and only the curve.
fn decaying(curve: f32) -> Patch {
    Patch {
        amp: Adsr {
            curve,
            ..adsr(0.0, 1.0, 0.0, 0.0)
        },
        ..saw_patch()
    }
}

/// A sweep of `semitones` arriving on the played note over `d` seconds.
fn sweep(semitones: f32, d: f32) -> PitchEnv {
    PitchEnv {
        semitones,
        adsr: adsr(0.0, d, 0.0, 0.0),
    }
}

/// The pitch a window is at, as rising zero crossings — for a saw this *is*
/// its frequency, so no FFT is needed to say the note moved.
fn hz(buf: &[f32], from: usize) -> usize {
    rising_crossings(&buf[from..from + WINDOW])
}

/// A downward sweep starts the note above the pitch it was played at and
/// leaves it there — the whole gesture, measured as pitch rather than as
/// "something changed".
#[test]
fn a_pitch_envelope_starts_the_note_high_and_lands_it_on_the_played_pitch() {
    let mut patch = saw_patch();
    patch.pitch_env = Some(sweep(12.0, 0.5));
    let swept = render(&patch, 45.0, &opts(1.0));
    let plain = render(&saw_patch(), 45.0, &opts(1.0));

    assert!(
        hz(&swept, 0) > hz(&plain, 0) + 4,
        "the sweep starts high: {} vs {}",
        hz(&swept, 0),
        hz(&plain, 0)
    );
    assert_eq!(
        hz(&swept, 39_690),
        hz(&plain, 39_690),
        "and has arrived on the note by the end"
    );
}

/// Negative goes the other way. Stated separately from the positive case
/// because a sign that was ignored would pass a test that only asked whether
/// the pitch moved.
#[test]
fn a_negative_sweep_starts_the_note_low_instead() {
    let mut patch = saw_patch();
    patch.pitch_env = Some(sweep(-12.0, 0.5));
    let swept = render(&patch, 45.0, &opts(1.0));
    let plain = render(&saw_patch(), 45.0, &opts(1.0));
    assert!(
        hz(&swept, 0) < hz(&plain, 0),
        "the sweep starts low: {} vs {}",
        hz(&swept, 0),
        hz(&plain, 0)
    );
}

/// A sweep and a vibrato are both audible at once. The sum happens in
/// semitones inside one track, so a version where the last modulator written
/// won would lose one of these two assertions.
#[test]
fn a_sweep_and_a_vibrato_are_both_audible_on_one_note() {
    let mut patch = saw_patch();
    patch.pitch_env = Some(sweep(12.0, 0.5));
    patch.lfo = Some(Lfo {
        rate: 6.0,
        depth: 2.0,
        target: LfoTarget::Pitch,
    });
    let both = render(&patch, 45.0, &opts(1.0));

    let mut swept_only = saw_patch();
    swept_only.pitch_env = patch.pitch_env;
    let swept = render(&swept_only, 45.0, &opts(1.0));

    assert!(hz(&both, 0) > hz(&swept, 39_690), "the sweep still sweeps");
    assert_ne!(
        hz(&both, 39_690),
        hz(&swept, 39_690),
        "and the vibrato is still wobbling once the sweep has settled"
    );
}

/// The point of curving at all: partway through, a bent decay has already shed
/// what a straight one is still holding on to.
#[test]
fn a_curved_decay_is_further_down_than_a_straight_one() {
    let line = render(&decaying(0.0), 60.0, &opts(1.0));
    let bent = render(&decaying(4.0), 60.0, &opts(1.0));

    assert!(
        peak(&bent[22_050..22_050 + WINDOW]) < peak(&line[22_050..22_050 + WINDOW]) * 0.5,
        "halfway down: {} vs {}",
        peak(&bent[22_050..22_050 + WINDOW]),
        peak(&line[22_050..22_050 + WINDOW])
    );
    assert!(
        (peak(&bent[..64]) - peak(&line[..64])).abs() < 1e-3,
        "but the strike itself is as loud as it was"
    );
}

/// And a negative curve is the other way about — it holds on to its level and
/// then goes, which is what makes it a swell rather than a decay. Stated as
/// its own case because a bend that ignored the sign would pass the test above
/// and still be wrong here.
#[test]
fn a_negative_curve_holds_its_level_instead_of_shedding_it() {
    let line = render(&decaying(0.0), 60.0, &opts(1.0));
    let held = render(&decaying(-4.0), 60.0, &opts(1.0));
    assert!(
        peak(&held[22_050..22_050 + WINDOW]) > peak(&line[22_050..22_050 + WINDOW]) * 1.5,
        "halfway down: {} vs {}",
        peak(&held[22_050..22_050 + WINDOW]),
        peak(&line[22_050..22_050 + WINDOW])
    );
}

/// No bend, either way, may turn the amp stage into silence or into a buffer
/// with a `NaN` in it. That a curve lands exactly on its destination is
/// `core::env`'s to prove; this is the guard that the stage still renders a
/// note at all.
#[test]
fn no_curve_renders_silence_or_a_non_finite_sample() {
    for curve in [-8.0, -3.0, 0.0, 3.0, 8.0] {
        let buf = render(&decaying(curve), 60.0, &opts(1.0));
        assert!(peak(&buf[..WINDOW]) > 0.5, "curve {curve} rendered nothing");
        assert!(
            buf.iter().all(|sample| sample.is_finite()),
            "curve {curve} went non-finite"
        );
    }
}
