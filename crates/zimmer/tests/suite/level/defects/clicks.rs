//! Where a click starts: each threshold met exactly by a jump built sample by
//! sample, and missed just under it.

use super::{RATE, found};

/// Silence that steps up to `level` half-way through and stays there: one
/// jump of exactly `level`, and nothing around it.
fn step_out_of_silence(level: f32) -> Vec<f32> {
    let mut signal = vec![0.0; RATE as usize / 2];
    signal[RATE as usize / 4..].fill(level);
    signal
}

/// The smallest click is −40 dBFS, a jump of a hundredth of full scale, and a
/// click out of silence is measured against the quietest jump there is,
/// −80 dBFS: 40 dB for a hundredth, 60 for a tenth.
#[test]
fn the_smallest_click_is_a_hundredth_of_full_scale() {
    for (level, size_db) in [(0.01, 40.0), (0.1, 60.0)] {
        let defects = found(1, &step_out_of_silence(level));
        assert_eq!(defects.clicks.count, 1, "{level}: {defects:?}");
        let click = &defects.clicks.worst[0];
        assert!((click.size_db - size_db).abs() < 0.01, "{level}: {click:?}");
        assert_eq!(click.at_seconds, 0.25, "{level}: {click:?}");
    }
    let under = found(1, &step_out_of_silence(0.0099));
    assert_eq!(under.clicks.count, 0, "{under:?}");
}

/// A zigzag at Nyquist, every jump a sixteenth, offset by `offset` from the
/// middle on: one jump of a sixteenth and `offset` among jumps of a sixteenth.
fn zigzag_stepped_by(offset: f32) -> Vec<f32> {
    let half = RATE as usize / 4;
    (0..2 * half)
        .map(|i| {
            let zig = if i % 2 == 0 { 1.0 / 32.0 } else { -1.0 / 32.0 };
            zig + if i >= half { offset } else { 0.0 }
        })
        .collect()
}

/// A click is four times every jump around it, twelve decibels, and the
/// size it reads is how far it stands above the largest of them.
#[test]
fn a_click_is_four_times_every_jump_around_it() {
    // A jump from −1/32 to 1/32 + 3/16: exactly 4 × 1/16, every value exact.
    let defects = found(1, &zigzag_stepped_by(3.0 / 16.0));
    assert_eq!(defects.clicks.count, 1, "{defects:?}");
    let click = &defects.clicks.worst[0];
    assert!(
        (click.size_db - 20.0 * 4f64.log10()).abs() < 0.01,
        "{click:?}"
    );
    assert_eq!(click.at_seconds, 0.25, "{click:?}");

    let under = found(1, &zigzag_stepped_by(0.18));
    assert_eq!(under.clicks.count, 0, "3.9 times: {under:?}");
}

/// Two jumps of twenty, ten milliseconds apart, with the waveform brought
/// back down smoothly between them: the first lands from 0 on 20, the second
/// from `before` on `after`. Values this large keep every one exact, and a
/// twentieth of 20 is exactly 1.
fn two_jumps(before: f32, after: f32) -> Vec<f32> {
    let (p, q) = (RATE as usize / 4, RATE as usize / 4 + 480);
    let mut signal = vec![0.0; RATE as usize / 2];
    let ramp = |from: f32, to: f32, at: usize, len: usize, signal: &mut [f32]| {
        for k in 0..len {
            signal[at + k] = from + (to - from) * k as f32 / len as f32;
        }
    };
    signal[p..p + 6].fill(20.0);
    ramp(20.0, before, p + 6, q - p - 12, &mut signal);
    signal[q - 6..q].fill(before);
    signal[q..q + 6].fill(after);
    ramp(after, 0.0, q + 6, 460, &mut signal);
    signal
}

/// A jump with a twin in reach is a repeating waveform's, not a click, and a
/// twin is the waveform either side matching to a twentieth of the jump: the
/// sample three before and the sample two after, each on its own.
#[test]
fn a_twin_matches_either_side_to_a_twentieth_of_the_jump() {
    let twins = found(1, &two_jumps(1.0, 21.0));
    assert_eq!(
        twins.clicks.count, 0,
        "a twentieth off either side: {twins:?}"
    );
    for (before, after) in [(1.5, 20.0), (0.0, 21.5)] {
        let defects = found(1, &two_jumps(before, after));
        assert!(
            defects.clicks.count >= 1,
            "{before} before, {after} after: {defects:?}"
        );
    }
}
