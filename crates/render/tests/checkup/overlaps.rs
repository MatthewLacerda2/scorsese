//! Two layers drawn over each other, and the arrangements that are not a
//! mistake.
//!
//! The filter is the feature, so most of these assert **silence** — and each
//! silent case is paired with the same geometry pushed until it is a collision,
//! so that a rule cannot pass by suppressing everything.

use std::path::Path;

use scorsese_core::{Asset, AssetId, Frames, HashCheck, Project, Rgba, TextStyle};
use scorsese_render::Checkup;

use crate::common::{clip, held, project, shape_asset, text_asset, video_track};

/// Only what this module is about, out of a whole checkup — so a fixture that
/// happens to warn about something else cannot make one of these pass.
fn collisions(project: &Project) -> Vec<String> {
    Checkup::of(project, Path::new("no-such-project.scor"), HashCheck::Skip)
        .lines()
        .iter()
        .filter(|line| line.says.contains("drawn over each other"))
        .map(|line| line.says.clone())
        .collect()
}

fn silent(project: &Project) {
    let said = collisions(project);
    assert!(said.is_empty(), "expected nothing, got {said:?}");
}

fn reported(project: &Project) {
    assert_eq!(collisions(project).len(), 1, "{:?}", collisions(project));
}

/// A caption of its own width, sized so that one line is a thin band.
fn caption(id: &str, width: f64) -> Asset {
    Asset {
        style: Some(TextStyle {
            max_width: width,
            size: 0.023,
            ..TextStyle::default()
        }),
        ..text_asset(id)
    }
}

/// Two half-frame panels on two tracks, each pushed 0.15 to one side: they
/// share a fifth of the frame's width for two seconds.
fn panels() -> Project {
    let aside = |id, by| held(clip(id, "panel", 0, 60), "transform.position.x", by);
    project(
        vec![shape_asset("panel", 0.5, 0.5)],
        vec![
            video_track("left", vec![aside("near", -0.15)]),
            video_track("right", vec![aside("far", 0.15)]),
        ],
    )
}

#[test]
fn two_panels_across_each_other_are_reported_with_the_span_and_the_amount() {
    assert_eq!(
        collisions(&panels()),
        [
            "clips `near` and `far` are drawn over each other from 0.00–2.00s (f0–60), \
             `far` on top — the overlap covers 40% of `near`"
        ]
    );
}

#[test]
fn a_cut_underneath_does_not_split_one_collision_into_two() {
    let mut project = panels();
    project
        .assets
        .push(Asset::color(AssetId::new("wash"), Rgba::WHITE));
    // A background arriving halfway cuts the timeline in two, and the pair
    // above it is one finding across both halves.
    project
        .tracks
        .insert(0, video_track("bg", vec![clip("plate", "wash", 30, 30)]));
    reported(&project);
    assert!(collisions(&project)[0].contains("0.00–2.00s (f0–60)"));
}

#[test]
fn a_layer_covering_the_whole_frame_is_a_background_and_says_nothing() {
    silent(&project(
        vec![
            Asset::color(AssetId::new("wash"), Rgba::WHITE),
            shape_asset("panel", 0.5, 0.5),
        ],
        vec![
            video_track("bg", vec![clip("plate", "wash", 0, 60)]),
            video_track("front", vec![clip("card", "panel", 0, 60)]),
        ],
    ));
}

/// A box with a line of text on the track above it, the text dropped `by` — at
/// zero it sits inside its box, which is the label-in-a-box arrangement.
fn labelled(by: f64) -> Project {
    project(
        vec![shape_asset("plaque", 0.95, 0.2), text_asset("words")],
        vec![
            video_track("boxes", vec![clip("frame", "plaque", 0, 60)]),
            video_track(
                "labels",
                vec![held(
                    clip("label", "words", 0, 60),
                    "transform.position.y",
                    by,
                )],
            ),
        ],
    )
}

#[test]
fn a_label_inside_its_box_is_silent_and_the_same_label_hanging_out_of_it_is_not() {
    silent(&labelled(0.0));
    reported(&labelled(0.08));
}

/// Two captions on one line, `apart` between the second one and the middle of
/// the frame.
fn captions(apart: f64) -> Project {
    let aside = |id: &str, by| held(clip(id, id, 0, 60), "transform.position.x", by);
    project(
        vec![caption("near", 0.4), caption("far", 0.34)],
        vec![
            video_track("left", vec![aside("near", 0.02)]),
            video_track("right", vec![aside("far", apart)]),
        ],
    )
}

#[test]
fn two_captions_sharing_a_quarter_of_their_boxes_are_words_with_air_between_them() {
    // A wrap box is `max_width` wide whatever the words come to, so a quarter
    // of one shared is no evidence that a letter is over a letter.
    silent(&captions(0.3));
    reported(&captions(0.15));
}

/// A plate with a badge of `badge` on the track above it, both centred.
fn plated(plate: f64, badge: f64) -> Project {
    project(
        vec![
            shape_asset("plate", plate, plate),
            shape_asset("badge", badge, badge),
        ],
        vec![
            video_track("plates", vec![clip("back", "plate", 0, 60)]),
            video_track("badges", vec![clip("front", "badge", 0, 60)]),
        ],
    )
}

#[test]
fn a_badge_an_order_smaller_than_its_plate_is_decoration_on_it_and_says_nothing() {
    // Shapes rather than text, so nothing here turns on how a block of words is
    // boxed: the badge is wholly inside the plate in both cases, and only the
    // size difference tells them apart.
    silent(&plated(0.8, 0.1));
    reported(&plated(0.14, 0.1));
}

#[test]
fn a_two_frame_crossover_is_a_transition_and_not_a_collision() {
    let mut project = panels();
    project.tracks[0].clips[0].duration = Frames(30);
    project.tracks[1].clips[0].start = Frames(28);
    project.tracks[1].clips[0].duration = Frames(30);
    silent(&project);
}
