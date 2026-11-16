//! The heavy end (#1794): the distorted guitar and the driven bass that the
//! metal references, BFG Division and Brutal Doom, are played on.

use super::*;
use scorsese_zimmer::SAMPLE_RATE;

/// The low E string, where `kit:guitar` is at home.
const E2: f32 = 82.407;

/// The sustained middle of a one-second note at its home pitch, mixed to mono.
fn held_note(name: &str, velocity: f32) -> Vec<f32> {
    let opts = NoteOpts {
        duration: 1.0,
        velocity,
        ..NoteOpts::default()
    };
    let instrument = lookup(name).expect(name);
    let bake = bake_named_note(&instrument.patch(), instrument.home, &opts).expect(name);
    // 44 header bytes, then interleaved 16-bit stereo frames.
    let frames: Vec<f32> = bake.wav[44..]
        .chunks_exact(4)
        .map(|frame| {
            let left = i16::from_le_bytes([frame[0], frame[1]]);
            let right = i16::from_le_bytes([frame[2], frame[3]]);
            (f32::from(left) + f32::from(right)) / 2.0
        })
        .collect();
    let rate = SAMPLE_RATE as usize;
    frames[rate / 5..rate * 4 / 5].to_vec()
}

/// The power at one frequency, under a Hann window (Goertzel).
fn power_at(samples: &[f32], hz: f32) -> f32 {
    let n = samples.len() as f32;
    let coefficient = 2.0 * (std::f32::consts::TAU * hz / SAMPLE_RATE as f32).cos();
    let (mut previous, mut before) = (0.0_f32, 0.0_f32);
    for (index, sample) in samples.iter().enumerate() {
        let window = 0.5 - 0.5 * (std::f32::consts::TAU * index as f32 / n).cos();
        let current = sample * window + coefficient * previous - before;
        before = previous;
        previous = current;
    }
    previous * previous + before * before - coefficient * previous * before
}

/// One note of the guitar is a power chord: the fifth is in it. Every partial
/// of a root, a fifth and an octave — and of what the drive makes of their
/// sum — lands on a half-multiple of the root, so a quarter-step off that
/// grid (the major third's 1.25) is the floor to stand clear of.
#[test]
fn a_guitar_note_is_a_power_chord() {
    let guitar = held_note("guitar", 0.8);
    let fifth = power_at(&guitar, E2 * 1.5);
    let off_grid = power_at(&guitar, E2 * 1.25);
    assert!(
        fifth > 30.0 * off_grid,
        "the fifth ({fifth:.3e}) should stand far over the floor ({off_grid:.3e})"
    );
    let root = power_at(&guitar, E2);
    assert!(
        fifth > 0.1 * root,
        "the fifth ({fifth:.3e}) should be a voice beside the root ({root:.3e}), not a stray partial"
    );
}

/// Palm mute from velocity: a quiet note is darker than a hard one, which is
/// what makes `X-xxX-xx` a gallop rather than eight equal chugs.
#[test]
fn a_quiet_guitar_note_is_palm_muted() {
    let high_share = |velocity: f32| {
        let samples = held_note("guitar", velocity);
        let bright = power_at(&samples, 2_000.0) + power_at(&samples, 3_000.0);
        let body = power_at(&samples, E2) + power_at(&samples, E2 * 2.0);
        bright / body
    };
    assert!(high_share(1.0) > 2.0 * high_share(0.35));
}

/// What the "sits" column of `docs/recipes.md` says of the two.
#[test]
fn the_heavy_end_sits_where_its_names_say() {
    let bands = |name: &str| bands_of(lookup(name).expect(name));
    let (low, mid, _) = bands("guitar");
    assert!(
        low.abs_diff(mid) <= 25,
        "guitar: low and mid about even, {:?}",
        bands("guitar")
    );
    assert!(bands("growl").0 > 75, "growl: {:?}", bands("growl"));
}
