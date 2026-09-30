//! A shape whose line is keyframed, through the whole pipeline.
//!
//! The compositor's own tests say what a trimmed line looks like. What only a
//! render can say is that a keyframed trim is *read every frame* — a shape is
//! otherwise drawn once for a whole segment, and one drawn once here would show
//! the same picture from the first frame to the last with every test of the
//! drawing still green.

use scorsese_core::{
    Asset, AssetId, Easing, Fps, Frames, Geometry, Heads, Keyframe, KeyframeTrack, Point,
    PropertyPath, Rgba, Shape,
};
use scorsese_render::FrameRange;

use crate::common::ffmpeg::{fixture_dir, mean_rgb, tools};
use crate::common::{clip, project, video_track};
use crate::{colour_asset, render};

/// A plain white line straight across the middle of the frame.
fn line() -> Asset {
    let geometry = Geometry::Arrow {
        from: Point::new(0.0, 0.5).into(),
        to: Point::new(1.0, 0.5).into(),
        curve: Default::default(),
        heads: Heads::None,
    };
    let shape = Shape::outlined(geometry, Rgba::WHITE).bordered(Rgba::WHITE, 0.2);
    Asset::shape(AssetId::new("line"), shape)
}

fn drawn_on(from: u64, to: u64) -> KeyframeTrack {
    let key = |t: u64, value: f64| Keyframe {
        t: Frames(t),
        value,
        easing: Easing::Linear,
    };
    KeyframeTrack::new(
        PropertyPath::new("shape.trim_end"),
        vec![key(from, 0.0), key(to, 1.0)],
    )
}

/// The line draws itself on across the clip: nothing on the first frame, half
/// of it half way, all of it at the end. Measured as how much the red bed's
/// green is lifted, which is how much white is over it.
#[test]
fn a_keyframed_trim_is_drawn_again_on_every_frame() {
    let tools = tools();
    let dir = fixture_dir("trace");
    let red = colour_asset(&tools, &dir, "red", "64x64", 1);
    let mut drawing = clip("c2", "line", 0, 15);
    drawing.keyframes.push(drawn_on(0, 14));
    let project = project(
        vec![red, line()],
        vec![
            video_track("v1", vec![clip("c1", "red", 0, 15)]),
            video_track("v2", vec![drawing]),
        ],
    );

    let (out, report) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);
    assert!(report.notes.is_empty(), "{:?}", report.notes);
    let green = |frame| mean_rgb(&tools, &out, frame).1;
    let (first, half, last) = (green(0), green(7), green(14));
    std::fs::remove_dir_all(&dir).ok();

    assert!(first <= 2, "nothing drawn yet: {first}");
    assert!(half > first + 4, "half the line by the middle: {half}");
    assert!(last > half + 4, "and all of it by the end: {last}");
}
