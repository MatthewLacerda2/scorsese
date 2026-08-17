//! An icon clip through the whole pipeline: no file, and a name to resolve.
//!
//! The golden fixture pins what a symbol *looks* like. What only a pipeline
//! test can say is what happens either side of that: that an icon clip renders
//! with nothing on disk behind it, and that a name the build does not ship
//! leaves the render standing rather than stopping it — with a note, because an
//! empty layer is otherwise invisible to anything counting frames.

use scorsese_core::{Asset, AssetId, Fps, Icon, Rgba};
use scorsese_render::{FrameRange, Note};

use crate::common::ffmpeg::{fixture_dir, inspect, mean_rgb, tools};
use crate::common::{clip, project, video_track};
use crate::{colour_asset, render};

/// Nearly filling the 64×64 raster, so a symbol that draws lifts the frame's
/// mean well clear of the bed's and one that does not leaves it untouched.
fn badge(name: &str) -> Asset {
    Asset::icon(
        AssetId::new("badge"),
        Icon::new(name, 0.85, Rgba::new(0xff, 0xff, 0xff, 0xff)),
    )
}

/// A white symbol over a red bed, and the render's report.
fn over_red(label: &str, name: &str) -> (u8, Vec<Note>) {
    let tools = tools();
    let dir = fixture_dir(label);
    let red = colour_asset(&tools, &dir, "red", "64x64", 1);
    let project = project(
        vec![red, badge(name)],
        vec![
            video_track("v1", vec![clip("c1", "red", 0, 15)]),
            video_track("v2", vec![clip("c2", "badge", 0, 15)]),
        ],
    );

    let (out, report) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);
    assert_eq!(report.frames, 15);
    assert_eq!(inspect(&tools, &out).frames, 15);
    let (_, green, _) = mean_rgb(&tools, &out, 7);
    std::fs::remove_dir_all(&dir).ok();
    (green, report.notes)
}

/// The whole bargain of an inline kind: nothing is decoded for it, and the
/// render still writes every frame.
#[test]
fn an_icon_clip_renders_without_a_file_to_decode() {
    let (green, notes) = over_red("icon", "clapperboard");
    assert!(
        notes.is_empty(),
        "a symbol that exists says nothing: {notes:?}"
    );
    // The bed is red, so any green at all is white ink over it.
    assert!(
        green > 4,
        "white strokes lift a red frame's green; found {green}"
    );
}

/// One mistyped string must not cost a whole preview — but it may not pass in
/// silence either, because the layer it leaves behind is empty.
#[test]
fn a_name_this_build_does_not_ship_notes_it_and_carries_on() {
    let (green, notes) = over_red("icon-unknown", "clapperbord");
    assert!(
        notes.iter().any(|note| matches!(
            note,
            Note::UnknownIcon { clip, asset, named }
                if clip == "c2" && asset == "badge" && named == "clapperbord"
        )),
        "an unknown name is worth saying: {notes:?}"
    );
    assert_eq!(green, 0, "nothing was drawn, so the bed is untouched");
}
