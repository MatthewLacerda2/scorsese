//! The clips there is no rectangle for, and the four different reasons.
//!
//! Said rather than left out, which is the whole of the design here: "not on
//! screen at this instant" and "on screen and immeasurable" are answers a
//! caller acts on differently, and an omission is neither.

use std::path::Path;

use scorsese_core::{Asset, AssetId, AssetKind, Endpoint, Frames, Geometry, Point, Rgba, Shape};
use scorsese_render::{Absence, Layout};

use super::common::{audio_track, clip, file_asset, project, text_asset, video_track};
use super::{layout, raster};

/// Why a clip has no rectangle here.
fn why(layout: &Layout, clip: &str) -> Absence {
    layout
        .unplaced
        .iter()
        .find(|unplaced| unplaced.clip == clip)
        .unwrap_or_else(|| panic!("`{clip}` is accounted for"))
        .why
        .clone()
}

/// The sentence an [`Absence::Unknown`] carries.
fn sentence(layout: &Layout, clip: &str) -> String {
    match why(layout, clip) {
        Absence::Unknown(said) => said,
        other => panic!("`{clip}` was answered for as {other:?}"),
    }
}

#[test]
fn a_clip_that_runs_later_is_off_screen_rather_than_missing() {
    let project = project(
        vec![text_asset("words")],
        vec![video_track(
            "titles",
            vec![
                clip("first", "words", 0, 30),
                clip("second", "words", 30, 30),
            ],
        )],
    );
    assert_eq!(why(&layout(&project, 0), "second"), Absence::OffScreen);
    assert_eq!(why(&layout(&project, 30), "first"), Absence::OffScreen);
}

#[test]
fn a_sound_playing_here_is_a_different_answer_from_one_that_is_not() {
    let project = project(
        vec![text_asset("words"), file_asset("score", AssetKind::Audio)],
        vec![
            video_track("titles", vec![clip("card", "words", 0, 60)]),
            audio_track("music", vec![clip("bed", "score", 0, 30)]),
        ],
    );
    assert_eq!(why(&layout(&project, 0), "bed"), Absence::Sound);
    assert_eq!(why(&layout(&project, 40), "bed"), Absence::OffScreen);
}

#[test]
fn an_arrow_says_it_has_no_box_rather_than_claiming_the_frame() {
    let project = project(
        vec![Asset::shape(
            AssetId::new("pointer"),
            Shape::outlined(
                Geometry::Arrow {
                    from: Endpoint::At(Point::new(0.1, 0.1)),
                    to: Endpoint::At(Point::new(0.4, 0.4)),
                    curve: scorsese_core::Curve::default(),
                    heads: scorsese_core::Heads::default(),
                },
                Rgba::WHITE,
            ),
        )],
        vec![video_track("diagram", vec![clip("line", "pointer", 0, 30)])],
    );
    assert!(
        sentence(&layout(&project, 0), "line").contains("arrow"),
        "an arrow's absence names itself"
    );
}

/// A project directory with an empty file where an asset says its media is.
/// Nothing decodes it — what is being tested is what the *document* records
/// about it.
fn with_media(id: &str) -> std::path::PathBuf {
    let dir = super::common::ffmpeg::fixture_dir("layout");
    std::fs::create_dir_all(dir.join("assets")).expect("create assets/");
    std::fs::write(dir.join(format!("assets/{id}.mp4")), []).expect("write the fixture file");
    dir
}

#[test]
fn a_source_nobody_measured_is_named_rather_than_guessed_at() {
    let root = with_media("shot");
    let project = project(
        vec![file_asset("shot", AssetKind::Video)],
        vec![video_track("picture", vec![clip("open", "shot", 0, 30)])],
    );
    let layout = Layout::at(&project, &root, raster(), Frames(0)).expect("the timeline sequences");
    assert!(
        sentence(&layout, "open").contains("probe"),
        "it says what to do about it"
    );
}

#[test]
fn a_missing_file_costs_that_clip_its_answer_and_not_the_frame() {
    let project = project(
        vec![text_asset("words"), file_asset("gone", AssetKind::Video)],
        vec![
            video_track("picture", vec![clip("open", "gone", 0, 30)]),
            video_track("titles", vec![clip("card", "words", 0, 30)]),
        ],
    );
    let layout = Layout::at(&project, Path::new("nowhere"), raster(), Frames(0))
        .expect("a missing file is one clip's problem");
    assert!(matches!(why(&layout, "open"), Absence::Unknown(_)));
    assert!(layout.of("card").is_some(), "the rest of the frame answers");
}
