//! With no knob turned, ducking writes exactly what it wrote before the knobs
//! existed (#1170).
//!
//! Every expected point below was captured from `main` before `between`,
//! `min_gap`, clips and ranges were added, on the narration timing of two
//! delivered videos and on the clip's edges. A change to any of them is a
//! change to every project already ducked.

use crate::ducking::{audio_track, clip, dip};
use scorsese_core::{Dip, Easing, Fps, Project, PropertyPath, TrackId, Under, duck_track};

/// A music clip of `length` frames under narration at `(from, to)` frames.
fn ducked(length: u64, lines: &[(u64, u64)], dip: Dip) -> Vec<(u64, f64, Easing)> {
    let mut project = Project::new("defaults", Fps::THIRTY);
    let narration = lines
        .iter()
        .enumerate()
        .map(|(i, (from, to))| clip(&format!("v{i}"), *from, to - from))
        .collect();
    project.tracks = vec![
        audio_track("music", vec![clip("m1", 0, length)]),
        audio_track("vo", narration),
    ];
    let under = Under::new(TrackId::new("music"));
    duck_track(&mut project, &under, &PropertyPath::new("volume"), dip).expect("ducks");
    project.tracks[0].clips[0].keyframes[0]
        .keyframes
        .iter()
        .map(|k| (k.t.get(), k.value, k.easing))
        .collect()
}

/// The Stories ad's four lines, ~0.3 s apart: one dip, as before.
#[test]
fn the_stories_ad_is_still_one_dip() {
    use Easing::{EaseIn, EaseOut, Linear};
    let lines = [(4, 66), (77, 187), (199, 326), (332, 396)];
    assert_eq!(
        ducked(450, &lines, dip()),
        vec![
            (0, 1.0, EaseOut),
            (4, 0.25, Linear),
            (396, 0.25, EaseIn),
            (414, 1.0, Linear)
        ]
    );
}

/// hivemind's sixteen lines, 0.7 to 1.1 s apart: the 22-frame pauses merge,
/// the 32-frame ones breathe — exactly as before.
#[test]
fn hivemind_merges_and_breathes_where_it_did() {
    let lines = [
        (24, 302),
        (324, 659),
        (680, 983),
        (1016, 1401),
        (1423, 1704),
        (1725, 1986),
        (2018, 2550),
        (2572, 3058),
        (3079, 3411),
        (3443, 3606),
        (3627, 4195),
        (4217, 4423),
        (4454, 4959),
        (4981, 5361),
        (5393, 5608),
        (5630, 5799),
    ];
    let points: Vec<(u64, f64)> = ducked(5850, &lines, dip())
        .into_iter()
        .map(|(t, value, _)| (t, value))
        .collect();
    #[rustfmt::skip]
    let expected = vec![
        (15, 1.0), (24, 0.25), (983, 0.25), (1001, 1.0),
        (1007, 1.0), (1016, 0.25), (1986, 0.25), (2004, 1.0),
        (2009, 1.0), (2018, 0.25), (3411, 0.25), (3429, 1.0),
        (3434, 1.0), (3443, 0.25), (4423, 0.25), (4441, 1.0),
        (4445, 1.0), (4454, 0.25), (5361, 0.25), (5379, 1.0),
        (5384, 1.0), (5393, 0.25), (5799, 0.25), (5817, 1.0),
    ];
    assert_eq!(points, expected);
}

/// A line at frame 0 and one running past the clip's end clamp as before.
#[test]
fn the_clip_edges_clamp_as_they_did() {
    use Easing::{EaseIn, EaseOut, Linear};
    assert_eq!(
        ducked(600, &[(0, 40), (300, 340), (580, 620)], dip()),
        vec![
            (0, 0.25, Linear),
            (40, 0.25, EaseIn),
            (58, 1.0, Linear),
            (291, 1.0, EaseOut),
            (300, 0.25, Linear),
            (340, 0.25, EaseIn),
            (358, 1.0, Linear),
            (571, 1.0, EaseOut),
            (580, 0.25, Linear),
            (600, 0.25, EaseIn)
        ]
    );
}

/// The defaults the CLI and MCP spell out are the dip core plans by default.
#[test]
fn spelling_the_defaults_out_is_the_default() {
    let spelled = Dip {
        over: Dip::BETWEEN,
        min_gap: None,
        ..Dip::from_seconds(Dip::UNDER, Dip::ATTACK_SECONDS, Dip::RELEASE_SECONDS, 30.0)
    };
    assert_eq!(spelled, dip());
    assert_eq!(dip().gap_worth_keeping().get(), 27, "release plus attack");
}
