//! How the answer reads, which is the whole of the interface for a caller that
//! is being handed prose rather than a struct.

use scorsese_core::{AssetId, Geometry, Rgba, Shape};

use super::common::{clip, project, text_asset, video_track};
use super::layout;

/// A frame with a box on it, a title on it, and a title that runs later.
fn cut() -> scorsese_core::Project {
    project(
        vec![
            scorsese_core::Asset::shape(
                AssetId::new("panel"),
                Shape::filled(
                    Geometry::Rectangle {
                        width: 0.5,
                        height: 0.25,
                        radius: 0.0,
                    },
                    Rgba::WHITE,
                ),
            ),
            text_asset("words"),
        ],
        vec![
            video_track("diagram", vec![clip("box", "panel", 0, 60)]),
            video_track(
                "titles",
                vec![
                    clip("caption", "words", 0, 30),
                    clip("later", "words", 30, 30),
                ],
            ),
        ],
    )
}

#[test]
fn it_says_the_instant_the_frame_and_that_the_numbers_are_fractions() {
    let said = layout(&cut(), 0).to_string();
    assert!(said.contains("0.00s (frame 0)"), "{said}");
    assert!(said.contains("1920x1080"), "{said}");
    assert!(said.contains("fractions"), "{said}");
}

#[test]
fn each_layer_is_one_line_of_track_clip_kind_and_rectangle() {
    let said = layout(&cut(), 0).to_string();
    assert!(
        said.contains("diagram/box") && said.contains("shape"),
        "{said}"
    );
    assert!(
        said.contains("x 0.250–0.750  y 0.375–0.625  (0.500 × 0.250)"),
        "{said}"
    );
    assert!(said.contains("titles/caption"), "{said}");
}

#[test]
fn the_clips_it_cannot_answer_for_are_in_the_words_and_not_only_in_the_data() {
    let said = layout(&cut(), 0).to_string();
    assert!(said.contains("not on screen here: later"), "{said}");
}

#[test]
fn a_gap_says_so_rather_than_listing_nothing() {
    let mut project = cut();
    project.tracks[0].clips[0].duration = scorsese_core::Frames(30);
    project.tracks[1].clips.pop();
    project.tracks[1].clips[0].duration = scorsese_core::Frames(30);
    project.tracks[0].clips.push(clip("tail", "panel", 60, 30));
    let said = layout(&project, 45).to_string();
    assert!(said.contains("nothing is on screen here"), "{said}");
}
