//! Where the `volume` property becomes a multiplier.

use scorsese_core::{KeyframeTrack, PropertyPath};
use scorsese_render::audio::Gain;

use crate::{keyframed, volume};

/// The multiplier `points` gives at `t` frames into the clip.
fn at(track: &KeyframeTrack, t: f64) -> f32 {
    Gain::of(std::slice::from_ref(track)).at(t)
}

#[test]
fn a_clip_with_no_volume_track_plays_at_its_own_level() {
    let none = Gain::of(&[]);
    assert!(none.is_unity(), "nothing to multiply by");
    assert_eq!(none.at(7.5), 1.0);
}

#[test]
fn only_the_volume_property_is_read() {
    // An opacity ramp on the same clip is the compositor's business. A mixer
    // that took the first track it found would fade the sound with the picture.
    let opacity = keyframed("opacity", &[(0, 0.0), (4, 1.0)]);
    let gain = Gain::of(std::slice::from_ref(&opacity));
    assert!(gain.is_unity(), "there is no volume track here");
    assert_eq!(
        gain.at(2.0),
        1.0,
        "and the clip is untouched, not half-faded"
    );
}

#[test]
fn a_volume_track_with_no_keyframes_animates_nothing() {
    // An empty track is a property nobody set, not a property set to zero —
    // silencing the clip would be inventing a value out of an absence.
    let empty = KeyframeTrack::new(PropertyPath::new("volume"), Vec::new());
    assert_eq!(at(&empty, 0.0), 1.0);
}

#[test]
fn the_multiplier_is_the_keyframe_line_read_at_that_frame() {
    // Silent at the clip's first frame, full four frames later: at frame one
    // the line is one quarter up, and one quarter is the whole assertion.
    let ramp = volume(&[(0, 0.0), (4, 1.0)]);
    assert_eq!(at(&ramp, 0.0), 0.0);
    assert_eq!(at(&ramp, 1.0), 0.25);
    assert_eq!(at(&ramp, 2.0), 0.5);
    assert_eq!(at(&ramp, 4.0), 1.0);
    assert_eq!(at(&ramp, 9.0), 1.0, "and it holds past the last keyframe");
}

#[test]
fn between_two_frames_the_ramp_travels_rather_than_stepping() {
    // A sample lands 48,000 times a second on a grid ruled 30 times a second,
    // so most of them fall between two frames. Snapping each to its nearest
    // whole frame turns a smooth fade into thirty steps a second — inaudible as
    // pitch, audible as a zipper.
    let ramp = volume(&[(0, 0.0), (4, 1.0)]);
    assert_eq!(at(&ramp, 1.5), 0.375, "half way from 0.25 to 0.5");
    assert_eq!(at(&ramp, 2.25), 0.5625);
    assert_eq!(at(&ramp, 3.75), 0.9375);
}

#[test]
fn a_ramp_downwards_is_read_the_same_way() {
    // The interpolation is `from + (to - from) * fraction`, and a fade-out is
    // where getting either end of that subtraction backwards shows up.
    let fade = volume(&[(0, 1.0), (4, 0.0)]);
    assert_eq!(at(&fade, 1.0), 0.75);
    assert_eq!(at(&fade, 1.5), 0.625);
    assert_eq!(at(&fade, 3.0), 0.25);
}

#[test]
fn a_volume_line_dragged_below_zero_is_silence_and_not_a_phase_flip() {
    let under = volume(&[(0, -2.0), (4, -1.0)]);
    assert_eq!(at(&under, 2.0), 0.0);
}

#[test]
fn time_before_the_clip_reads_the_clip_at_its_start() {
    // A negative clip-relative time is what a ramp asked about before its own
    // first frame means; it holds rather than extrapolating downwards.
    let ramp = volume(&[(0, 0.25), (4, 1.0)]);
    assert_eq!(at(&ramp, -3.0), 0.25);
}
