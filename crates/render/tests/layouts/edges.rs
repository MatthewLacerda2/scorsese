//! Words the frame had no room for, and content running off its edge (#1096):
//! reported on the placement, never refused.

use scorsese_core::{Asset, AssetId, Clip, Project, TextStyle};
use scorsese_render::{Overhang, Overset};

use super::common::{held, project, shape_asset, text_asset, video_track};
use super::layout;

/// The one placement of the one clip in `project`, at frame 0.
fn placed(project: &Project) -> scorsese_render::Placement {
    layout(project, 0).placed.remove(0)
}

/// Four lines at a size where the frame holds two: 0.3 of its height at a
/// line height of 1.25 is 0.375 a line.
fn overlong() -> Project {
    let title = Asset {
        style: Some(TextStyle {
            size: 0.3,
            line_height: 1.25,
            ..TextStyle::default()
        }),
        ..Asset::text(AssetId::new("title"), "ONE\nTWO\nTHREE\nFOUR")
    };
    project(
        vec![title],
        vec![video_track(
            "titles",
            vec![Clip::at("card", "title", 0, 30)],
        )],
    )
}

/// A half-width box moved `x` of the frame to the right of centre.
fn box_at(x: f64) -> Project {
    let moved = held(Clip::at("box", "panel", 0, 30), "transform.position.x", x);
    project(
        vec![shape_asset("panel", 0.5, 0.25)],
        vec![video_track("diagram", vec![moved])],
    )
}

#[test]
fn an_overlong_title_says_how_many_lines_it_set_and_how_many_show() {
    let overlong = overlong();
    assert_eq!(
        placed(&overlong).overset,
        Some(Overset { set: 4, shown: 2 })
    );
    let said = layout(&overlong, 0).to_string();
    assert!(said.contains("is cut off: 4 lines set, 2 shown"), "{said}");
}

#[test]
fn a_title_that_fits_is_neither_cut_nor_off_the_frame() {
    let fits = project(
        vec![text_asset("title")],
        vec![video_track(
            "titles",
            vec![Clip::at("card", "title", 0, 30)],
        )],
    );
    let placement = placed(&fits);
    assert_eq!((placement.overset, placement.overhang), (None, None));
}

/// The box spans 0.75–1.25 of the width, so a quarter of the frame is past
/// its right edge, and the sentence says so in percent.
#[test]
fn a_box_half_off_the_right_runs_off_it_by_the_half_that_is_out() {
    let overhang = placed(&box_at(0.5)).overhang.expect("half of it is out");
    let round = |by: f64| (by * 1000.0).round() / 1000.0;
    assert_eq!(
        (
            round(overhang.left),
            round(overhang.right),
            overhang.top,
            overhang.bottom
        ),
        (0.0, 0.25, 0.0, 0.0)
    );
    assert_eq!(
        overhang.to_string(),
        "runs off the right of the frame by 25.0% of its width"
    );
}

/// Wholly outside is not seen at all — an exit, or not yet in.
#[test]
fn a_box_wholly_off_the_frame_is_not_leaving_it() {
    assert_eq!(placed(&box_at(1.0)).overhang, None);
}

/// Past both edges of an axis is cropped by the frame, the way a slow zoom
/// is, rather than hanging off one side of it.
#[test]
fn a_box_wider_than_the_frame_is_cropped_not_leaving() {
    let mut zoomed = box_at(0.0);
    zoomed.tracks[0].clips[0] = held(zoomed.tracks[0].clips[0].clone(), "transform.scale.x", 3.0);
    assert_eq!(placed(&zoomed).overhang, None);
}

/// The words, not the wrap block: `THE END` in a block 0.9 of the frame wide,
/// moved a tenth right, puts the block's edge past the frame's and the words
/// nowhere near it — until they are moved far enough to cross it.
#[test]
fn text_leaves_the_frame_by_its_words_not_its_wrap_block() {
    let moved = |x: f64| {
        let title = Asset {
            style: Some(TextStyle {
                max_width: 0.9,
                ..TextStyle::default()
            }),
            ..text_asset("title")
        };
        let card = held(Clip::at("card", "title", 0, 30), "transform.position.x", x);
        placed(&project(
            vec![title],
            vec![video_track("titles", vec![card])],
        ))
    };
    let nudged = moved(0.1);
    assert!(nudged.area.right() > 1.0, "the block is past the edge");
    assert_eq!(nudged.overhang, None);
    let crossing: Option<Overhang> = moved(0.5).overhang;
    assert!(
        crossing.is_some_and(|overhang| overhang.right > 0.0 && overhang.left == 0.0),
        "{crossing:?}"
    );
}
