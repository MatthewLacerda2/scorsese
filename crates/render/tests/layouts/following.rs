//! A clip travelling along an arrow is reported where it is drawn.
//!
//! The render moves a follower onto its line after every transform resolves;
//! a layout that read the transform alone would put a small dot in the middle
//! of the frame, and `check` would warn about overlaps it never makes. These
//! ask the same question the pipeline's `following` tests answer in pixels.

use scorsese_core::{
    Asset, AssetId, Attach, ClipId, Endpoint, Follow, Geometry, Heads, Point, Project, Rgba, Shape,
    Side,
};
use scorsese_render::Absence;

use super::common::{clip, held, project, shape_asset, video_track};
use super::{layout, region, rounded};

/// An arrow from `from` to `to`.
fn line(from: Endpoint, to: Endpoint) -> Asset {
    let geometry = Geometry::Arrow {
        from,
        to,
        curve: Default::default(),
        heads: Heads::None,
    };
    Asset::shape(AssetId::new("line"), Shape::outlined(geometry, Rgba::WHITE))
}

/// Straight across the middle, from a tenth of the way in to nine tenths.
fn across() -> Asset {
    line(Point::new(0.1, 0.5).into(), Point::new(0.9, 0.5).into())
}

/// A tenth-of-the-frame dot, held `progress` of the way along `c-line`, on
/// screen for frames 0..30 — and the line on screen for `line_at`.
fn scene(line: Asset, line_at: (u64, u64), progress: f64) -> Project {
    let mut dot = held(clip("c-dot", "dot", 0, 30), "follow.progress", progress);
    dot.follow = Some(Follow::new(ClipId::new("c-line")));
    project(
        vec![
            line,
            shape_asset("dot", 0.1, 0.1),
            shape_asset("box", 0.2, 0.2),
        ],
        vec![
            video_track("v1", vec![clip("c-line", "line", line_at.0, line_at.1)]),
            video_track("v2", vec![dot]),
        ],
    )
}

/// A rectangle's middle, to the thousandth.
fn middle(project: &Project, clip: &str) -> (f64, f64) {
    let (left, top, width, height) = rounded(region(project, 0, clip));
    let round = |value: f64| (value * 1000.0).round() / 1000.0;
    (round(left + width / 2.0), round(top + height / 2.0))
}

/// The issue's own case: a quarter of the way from 0.1 to 0.9 is 0.3.
#[test]
fn a_dot_a_quarter_along_a_straight_arrow_is_reported_a_quarter_along() {
    let project = scene(across(), (0, 30), 0.25);
    assert_eq!(middle(&project, "c-dot"), (0.3, 0.5));
    assert_eq!(
        rounded(region(&project, 0, "c-dot")).2,
        0.1,
        "its size is its own"
    );
}

/// An arrow between two fixed places is followed from the document when it
/// is not on screen — the render's rule, so the layout's too.
#[test]
fn an_arrow_off_screen_between_fixed_places_is_still_followed() {
    let project = scene(across(), (60, 30), 1.0);
    assert_eq!(middle(&project, "c-dot"), (0.9, 0.5));
}

/// An arrow attached to a box runs to wherever the box is, and the follower
/// at its far end meets the box's left side.
#[test]
fn an_arrow_attached_to_a_box_is_followed_to_the_box() {
    let tied = Endpoint::Attached {
        attach: Attach {
            clip: ClipId::new("c-box"),
            side: Side::Left,
        },
    };
    let mut project = scene(line(Point::new(0.1, 0.5).into(), tied), (0, 30), 1.0);
    project
        .tracks
        .push(video_track("v3", vec![clip("c-box", "box", 0, 30)]));
    assert_eq!(middle(&project, "c-dot"), (0.4, 0.5));
}

/// Attached to a box that is not on screen, the arrow has no line, the render
/// leaves the dot out — and the layout says so rather than placing it.
#[test]
fn a_follower_whose_attached_arrow_has_nothing_to_point_at_is_not_placed() {
    let tied = Endpoint::Attached {
        attach: Attach {
            clip: ClipId::new("c-box"),
            side: Side::Left,
        },
    };
    let mut project = scene(line(Point::new(0.1, 0.5).into(), tied), (0, 30), 0.5);
    project
        .tracks
        .push(video_track("v3", vec![clip("c-box", "box", 60, 30)]));
    let layout = layout(&project, 0);
    assert_eq!(layout.of("c-dot"), None);
    let why = layout
        .unplaced
        .iter()
        .find(|unplaced| unplaced.clip == "c-dot")
        .map(|unplaced| unplaced.why.clone());
    assert!(
        matches!(&why, Some(Absence::Unknown(said)) if said.contains("`c-line`")),
        "{why:?}"
    );
}
