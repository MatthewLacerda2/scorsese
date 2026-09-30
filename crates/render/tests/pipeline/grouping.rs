//! A group drawn as one layer: its members composited offscreen, then the
//! result composited with the group clip's own properties.

use scorsese_core::{Asset, AssetId, Fit, Fps, Frames, Geometry, Group, Rgba, Shape};
use scorsese_render::{Frame, RenderSettings, Renderer, Resolution};

use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{clip, fitted, held, project, video_track};
use crate::{RED, assert_colour, colour_asset};

const WIDTH: u32 = 160;
const HEIGHT: u32 = 90;

fn settings() -> RenderSettings {
    RenderSettings::new(
        Resolution::new(WIDTH, HEIGHT).expect("a legal raster"),
        Fps::THIRTY,
    )
}

/// The colour at one pixel of a composited frame.
fn at(frame: &Frame, x: u32, y: u32) -> (u8, u8, u8) {
    let i = ((y * WIDTH + x) * 4) as usize;
    let bytes = frame.bytes();
    (bytes[i], bytes[i + 1], bytes[i + 2])
}

fn square(id: &str, color: Rgba) -> Asset {
    let geometry = Geometry::Rectangle {
        width: 0.4,
        height: 0.4,
        radius: 0.0,
    };
    Asset::shape(AssetId::new(id), Shape::filled(geometry, color))
}

/// Two opaque squares overlapping, grouped, and the group faded to half: where
/// they overlap only the upper one shows — the group is one picture, faded
/// once — and it is exactly the colour of its own uncovered half.
#[test]
fn a_faded_group_does_not_show_its_members_through_each_other() {
    let red = held(clip("red-in", "red", 0, 10), "transform.position.x", -0.1);
    let blue = held(clip("blue-in", "blue", 0, 10), "transform.position.x", 0.1);
    let pair = Group::new(vec![
        video_track("g-red", vec![red]),
        video_track("g-blue", vec![blue]),
    ]);
    let shown = held(clip("c-pair", "pair", 0, 10), "opacity", 0.5);
    let project = project(
        vec![
            square("red", Rgba::new(220, 0, 0, 255)),
            square("blue", Rgba::new(0, 0, 220, 255)),
            Asset::group(AssetId::new("pair"), pair),
        ],
        vec![video_track("v1", vec![shown])],
    );
    project.validate().expect("valid");
    let dir = fixture_dir("group-faded");
    let still = Renderer::new(&tools(), settings())
        .still(&project, &dir, Frames(0))
        .expect("a frame of drawn layers");

    // Red spans x 32–96, blue 64–128: the overlap is 64–96.
    let (overlap, blue_alone, red_alone) =
        (at(&still, 80, 45), at(&still, 112, 45), at(&still, 40, 45));
    assert_colour(overlap, blue_alone, "the overlap, which is the blue alone");
    assert_colour(blue_alone, (0, 0, 110), "the blue at half");
    assert_colour(red_alone, (110, 0, 0), "the red at half");
    std::fs::remove_dir_all(&dir).ok();
}

/// A decoded member is read into a buffer and drawn into the group's canvas
/// like any drawn one, and the group clip's move takes it along: a red plate
/// filling the raster, moved half a frame right as a group, leaves the left
/// half black.
#[test]
fn a_decoded_member_moves_with_its_group() {
    let tools = tools();
    let dir = fixture_dir("group-decoded");
    let plate = Group::new(vec![video_track(
        "g-plate",
        vec![fitted(Fit::Fill, clip("plate", "red", 0, 10))],
    )]);
    let shown = held(clip("c-plate", "moved", 0, 10), "transform.position.x", 0.5);
    let project = project(
        vec![
            colour_asset(&tools, &dir, "red", "64x64", 1),
            Asset::group(AssetId::new("moved"), plate),
        ],
        vec![video_track("v1", vec![shown])],
    );
    project.validate().expect("valid");
    let still = Renderer::new(&tools, settings())
        .still(&project, &dir, Frames(5))
        .expect("frame 5 is inside the group");

    assert_colour(at(&still, 40, 45), (0, 0, 0), "the left half, uncovered");
    assert_colour(at(&still, 120, 45), RED, "the right half, the plate");
    std::fs::remove_dir_all(&dir).ok();
}
