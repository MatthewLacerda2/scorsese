//! Summing sources into one buffer of samples.

use std::slice::from_ref;

use scorsese_render::audio::{CHANNELS, Gain, Mix, Ramp};

use crate::volume;

/// A run of samples that never moves through a fade: every sample-frame is
/// asked about clip-relative time zero.
const STILL: Ramp = Ramp {
    first: 0.0,
    per_sample: 0.0,
};

/// One sample-frame per timeline frame, starting at the clip's own frame one —
/// so the ramp both starts somewhere and advances.
const ADVANCING: Ramp = Ramp {
    first: 1.0,
    per_sample: 1.0,
};

/// A source of `frames` sample-frames, every sample at full scale.
fn full_scale(frames: usize) -> Vec<f32> {
    vec![1.0; frames * CHANNELS]
}

#[test]
fn a_silent_buffer_is_as_long_as_it_was_asked_for() {
    let mix = Mix::silence(3);
    assert_eq!(mix.frames(), 3, "sample-frames, not samples");
    assert_eq!(mix.samples(), [0.0; 3 * CHANNELS]);
}

#[test]
fn two_sources_playing_at_once_are_added_sample_by_sample() {
    // The assertion this file exists for. A mix measured as loudness over a
    // window is satisfied by summing, by subtracting, and by taking whichever
    // source is louder; only the sum is mixing.
    let mut mix = Mix::silence(2);
    mix.add(&[0.25, 0.5, 0.75, 0.125], Gain::of(&[]), STILL);
    mix.add(&[0.5, 0.25, 0.125, 0.25], Gain::of(&[]), STILL);
    assert_eq!(mix.samples(), [0.75, 0.75, 0.875, 0.375]);
}

#[test]
fn a_source_shorter_than_the_stretch_leaves_the_rest_as_it_found_it() {
    // Running out of audio is silence for the remainder, not a shortened mix —
    // and not an overwrite of whatever else is playing there.
    let mut mix = Mix::silence(3);
    mix.add(&[0.5, 0.5, 0.25, 0.25], Gain::of(&[]), STILL);
    mix.add(&[0.25, 0.25], Gain::of(&[]), STILL);
    assert_eq!(mix.samples(), [0.75, 0.75, 0.25, 0.25, 0.0, 0.0]);
}

#[test]
fn a_kept_source_is_scaled_by_its_volume_at_that_instant() {
    // Full-scale samples in, so what comes out *is* the multiplier: the buffer
    // reads back the volume line itself, one sample-frame at a time.
    let ramp = volume(&[(0, 0.0), (4, 1.0)]);
    let mut mix = Mix::silence(3);
    mix.add(&full_scale(3), Gain::of(from_ref(&ramp)), ADVANCING);
    assert_eq!(mix.samples(), [0.25, 0.25, 0.5, 0.5, 0.75, 0.75]);
}

#[test]
fn both_channels_of_one_instant_are_at_the_same_point_in_the_fade() {
    // Advancing the fade per *sample* rather than per sample-frame would put
    // the two channels half a step apart — a fade that walks across the stereo
    // image as it goes.
    let ramp = volume(&[(0, 0.0), (4, 1.0)]);
    let mut mix = Mix::silence(2);
    mix.add(&full_scale(2), Gain::of(from_ref(&ramp)), ADVANCING);
    let samples = mix.samples();
    assert_eq!(
        samples[0], samples[1],
        "left and right of the first instant"
    );
    assert_eq!(samples[2], samples[3], "and of the second");
    assert_eq!(samples[0], 0.25);
}

#[test]
fn a_scaled_source_is_added_to_what_is_already_there() {
    // The gain path and the unity path are two branches of the same operation,
    // and both of them accumulate.
    let ramp = volume(&[(0, 0.5)]);
    let mut mix = Mix::silence(1);
    mix.add(&[0.25, 0.25], Gain::of(&[]), STILL);
    mix.add(&[0.5, 0.5], Gain::of(from_ref(&ramp)), STILL);
    assert_eq!(mix.samples(), [0.5, 0.5]);
}

#[test]
fn sums_past_full_scale_are_clipped_only_when_they_are_written_out() {
    let mut mix = Mix::silence(1);
    mix.add(&[0.75, -0.75], Gain::of(&[]), STILL);
    mix.add(&[0.75, -0.75], Gain::of(&[]), STILL);
    assert_eq!(
        mix.samples(),
        [1.5, -1.5],
        "the overshoot is carried, not lost"
    );

    let mut written = Vec::new();
    mix.write_to(&mut written)
        .expect("a Vec never fails to write");
    let out: Vec<f32> = written
        .chunks_exact(size_of::<f32>())
        .map(|bytes| f32::from_le_bytes(bytes.try_into().expect("four bytes")))
        .collect();
    assert_eq!(out, [1.0, -1.0], "clipped at both ends, and audibly so");
}
