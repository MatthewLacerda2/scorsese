//! Two volume tracks on one clip: a hand-written fade and a duck (#1099).

use scorsese_core::PropertyPath;
use scorsese_render::audio::{Gain, path::VOLUME};

use crate::arithmetic::{keyframed, volume};

#[test]
fn every_volume_track_multiplies_whichever_is_written_first() {
    // Half under a fade, a quarter under the duck: an eighth, either way round.
    let fade = volume(&[(0, 0.5)]);
    let duck = volume(&[(0, 0.25)]).generated_by("duck");
    for tracks in [[fade.clone(), duck.clone()], [duck, fade]] {
        let gain = Gain::of(&tracks);
        assert!(!gain.is_unity());
        assert_eq!(gain.at(3.0), 0.125, "both are heard");
    }
}

#[test]
fn each_track_travels_on_its_own_between_frames() {
    let up = volume(&[(0, 0.0), (4, 1.0)]);
    let half = volume(&[(0, 0.5)]);
    let tracks = [up, half];
    let gain = Gain::of(&tracks);
    assert_eq!(gain.at(1.5), 0.1875, "0.375 of the ramp, halved");
}

#[test]
fn two_lines_below_zero_stay_silent_rather_than_multiplying_into_gain() {
    let tracks = [volume(&[(0, -1.0)]), volume(&[(0, -2.0)])];
    let gain = Gain::of(&tracks);
    assert_eq!(gain.at(0.0), 0.0);
}

#[test]
fn another_property_beside_them_is_still_not_read() {
    let tracks = [volume(&[(0, 0.5)]), keyframed("opacity", &[(0, 0.0)])];
    let gain = Gain::of(&tracks);
    assert_eq!(gain.at(0.0), 0.5);
}

#[test]
fn the_format_lets_volume_stack() {
    // Validation refuses a second track on any path that does not stack, so a
    // mixer that multiplies has to be reading a path the format agrees on.
    assert!(PropertyPath::new(VOLUME).stacks());
}
