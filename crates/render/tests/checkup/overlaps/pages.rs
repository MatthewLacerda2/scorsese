//! A page over a picture: a rectangle that is a canvas, not a shape (#807).

use scorsese_core::{Asset, AssetId, AssetKind, Project, ProjectPath};

use super::{reported, silent};
use crate::common::{clip, held, project, shape_asset, video_track};

/// Two layers made of `asset` stacked on two tracks, each drawn at nine tenths
/// of the frame: just under the frame-filling line, so a pair of them is
/// judged by its overlap rather than waved through as a background.
pub(super) fn stacked(asset: Asset) -> Project {
    let near = |id: &str| {
        let shrunk = held(clip(id, "layer", 0, 60), "transform.scale.x", 0.9);
        held(shrunk, "transform.scale.y", 0.9)
    };
    project(
        vec![Asset {
            id: AssetId::new("layer"),
            ..asset
        }],
        vec![
            video_track("under", vec![near("stack")]),
            video_track("over", vec![near("embed")]),
        ],
    )
}

#[test]
fn a_page_says_nothing_about_what_it_hides_though_a_picture_the_same_size_does() {
    // A page is drawn with alpha, so its rectangle is a canvas and not what it
    // covers (#807): two full pages side by side each draw only their half.
    silent(&stacked(Asset::imported(
        AssetId::new("layer"),
        AssetKind::Html,
        ProjectPath::new("pages/half.html"),
    )));
    reported(&stacked(shape_asset("layer", 1.0, 1.0)));
}
