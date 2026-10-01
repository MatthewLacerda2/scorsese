//! Clips that travel along an arrow, through the whole pipeline.
//!
//! Measured as where a red square's pixels land in a still, so every test is a
//! claim about the picture rather than about a number on the way to it: where
//! on the line `follow.progress` puts the square, that its own position is an
//! offset from there, that it turns with `orient`, and what happens when the
//! line cannot be known.

use scorsese_core::{
    Asset, AssetId, Attach, ClipId, Endpoint, Follow, Fps, Frames, Geometry, Heads, Point, Project,
    Rgba, Shape, Side,
};
use scorsese_render::{Frame, FrameRange, Renderer};

use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{clip, held, project, shape_asset, video_track};
use crate::{RASTER, render, settings};

/// A thin, dark line from `from` to `to`, drawn but invisible against black.
fn line(from: Endpoint, to: Endpoint) -> Asset {
    let geometry = Geometry::Arrow {
        from,
        to,
        curve: Default::default(),
        heads: Heads::None,
    };
    let shape = Shape::outlined(geometry, Rgba::opaque(1, 1, 1)).bordered(Rgba::BLACK, 0.01);
    Asset::shape(AssetId::new("line"), shape)
}

/// A red rectangle `width` by `height`, in fractions of the raster.
fn block(id: &str, width: f64, height: f64) -> Asset {
    let mut asset = shape_asset(id, width, height);
    asset.shape.as_mut().expect("a shape").fill = Some(Rgba::opaque(255, 0, 0).into());
    asset
}

/// The red square, `progress` of the way along the line `c-line`.
fn follower(progress: f64) -> scorsese_core::Clip {
    let mut dot = held(clip("c-dot", "dot", 0, 30), "follow.progress", progress);
    dot.follow = Some(Follow::new(ClipId::new("c-line")));
    dot
}

/// Straight across the middle, from a tenth of the way in to nine tenths.
fn across() -> Asset {
    line(Point::new(0.1, 0.5).into(), Point::new(0.9, 0.5).into())
}

fn scene(line: Asset, line_clip: scorsese_core::Clip, dot: scorsese_core::Clip) -> Project {
    project(
        vec![line, block("dot", 0.125, 0.125)],
        vec![
            video_track("v1", vec![line_clip]),
            video_track("v2", vec![dot]),
        ],
    )
}

fn still(project: &Project, at: u64) -> Frame {
    let dir = fixture_dir("following");
    let frame = Renderer::new(&tools(), settings(Fps::THIRTY))
        .still(project, &dir, Frames(at))
        .expect("the frame is inside the edit");
    std::fs::remove_dir_all(&dir).ok();
    frame
}

/// The middle of the red pixels, and how wide and tall they spread.
fn red(frame: &Frame) -> Option<((f64, f64), (u32, u32))> {
    let (mut sum, mut count) = ((0.0, 0.0), 0.0);
    let (mut low, mut high) = ((u32::MAX, u32::MAX), (0, 0));
    for (at, pixel) in frame.bytes().chunks_exact(4).enumerate() {
        if pixel[0] < 128 || pixel[1] > 64 {
            continue;
        }
        let (x, y) = (at as u32 % RASTER.0, at as u32 / RASTER.0);
        sum = (sum.0 + f64::from(x) + 0.5, sum.1 + f64::from(y) + 0.5);
        count += 1.0;
        low = (low.0.min(x), low.1.min(y));
        high = (high.0.max(x), high.1.max(y));
    }
    (count > 0.0).then(|| {
        let spread = (high.0 - low.0 + 1, high.1 - low.1 + 1);
        ((sum.0 / count, sum.1 / count), spread)
    })
}

#[track_caller]
fn assert_near(found: (f64, f64), expected: (f64, f64)) {
    let close = (found.0 - expected.0).abs() <= 1.0 && (found.1 - expected.1).abs() <= 1.0;
    assert!(close, "the square is at {found:?}, not {expected:?}");
}

/// A quarter of the way along a line 0.8 of the frame long, from 0.1 in.
#[test]
fn progress_puts_the_middle_of_the_clip_on_the_line() {
    let frame = still(
        &scene(across(), clip("c-line", "line", 0, 30), follower(0.25)),
        5,
    );
    let (middle, _) = red(&frame).expect("the square is drawn");
    assert_near(middle, (64.0 * 0.3, 32.0));
}

/// A bob on top of a path is an offset from the point on the line.
#[test]
fn position_is_an_offset_from_the_line() {
    let dot = held(follower(0.5), "transform.position.y", 0.25);
    let frame = still(&scene(across(), clip("c-line", "line", 0, 30), dot), 5);
    let (middle, _) = red(&frame).expect("the square is drawn");
    assert_near(middle, (32.0, 48.0));
}

/// A line between fixed places is geometry: it is followed before its own clip
/// has started.
#[test]
fn an_arrow_not_yet_on_screen_is_followed_all_the_same() {
    let frame = still(
        &scene(across(), clip("c-line", "line", 20, 10), follower(1.0)),
        5,
    );
    let (middle, _) = red(&frame).expect("the square is drawn");
    assert_near(middle, (64.0 * 0.9, 32.0));
}

/// A bar lying along x, following a line that runs straight down, stands up.
#[test]
fn orient_turns_the_clip_to_face_along_the_line() {
    let down = line(Point::new(0.5, 0.1).into(), Point::new(0.5, 0.9).into());
    let mut bar = follower(0.5);
    bar.follow = Some(Follow {
        clip: ClipId::new("c-line"),
        orient: true,
    });
    let mut project = scene(down, clip("c-line", "line", 0, 30), bar);
    project.assets[1] = block("dot", 0.25, 0.0625);
    let (middle, (wide, tall)) = red(&still(&project, 5)).expect("the bar is drawn");
    assert_near(middle, (32.0, 32.0));
    assert!(
        tall > wide * 2,
        "turned to run down the line: {wide}×{tall}"
    );
}

/// An attached arrow off screen has no line to follow, so its follower is left
/// out — and the render says why.
#[test]
fn a_follower_of_an_attached_arrow_off_screen_is_left_out_and_reported() {
    let to_box = Endpoint::Attached {
        attach: Attach {
            clip: ClipId::new("c-box"),
            side: Side::Left,
        },
    };
    let mut project = scene(
        line(Point::new(0.1, 0.1).into(), to_box),
        clip("c-line", "line", 20, 10),
        follower(0.5),
    );
    project.assets.push(shape_asset("box", 0.1, 0.1));
    project
        .tracks
        .push(video_track("v3", vec![clip("c-box", "box", 0, 30)]));
    project.validate().expect("a valid document");
    assert!(red(&still(&project, 5)).is_none(), "the square is left out");

    let tools = tools();
    let dir = fixture_dir("following-lost");
    let (_, report) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);
    std::fs::remove_dir_all(&dir).ok();
    let said: Vec<String> = report.notes.iter().map(ToString::to_string).collect();
    let lost = said
        .iter()
        .filter(|note| note.contains("follows arrow `c-line`"));
    assert_eq!(
        lost.count(),
        1,
        "said once, for the stretch it was lost: {said:?}"
    );
}
