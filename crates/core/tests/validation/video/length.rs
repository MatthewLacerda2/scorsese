//! How long a shot is, and the choices that stop it being a choice.

use super::asking;
use crate::common::{assert_only_problem, asset_id};
use scorsese_core::{ClipSeconds, VideoProblem as V, VideoResolution};

#[test]
fn a_request_of_nothing_but_defaults_is_valid() {
    let p = asking(|_| {});
    assert!(p.is_valid(), "defaults alone are a complete request");
}

#[test]
fn ten_eighty_only_generates_at_eight_seconds() {
    let p = asking(|r| r.seconds = ClipSeconds::Four);
    assert_only_problem(
        &p,
        V::LengthLocked {
            asset: asset_id("shot-city"),
            asked: 4,
            cause: "1080p",
        },
    );
}

#[test]
fn a_shorter_shot_is_available_at_the_smaller_raster() {
    let p = asking(|r| {
        r.resolution = VideoResolution::P720;
        r.seconds = ClipSeconds::Four;
    });
    assert!(p.is_valid(), "720p leaves the length free to choose");
}

#[test]
fn reference_images_fix_the_length_even_at_the_smaller_raster() {
    let p = asking(|r| {
        r.resolution = VideoResolution::P720;
        r.seconds = ClipSeconds::Six;
        r.reference_images = vec![asset_id("logo")];
    });
    assert_only_problem(
        &p,
        V::LengthLocked {
            asset: asset_id("shot-city"),
            asked: 6,
            cause: "reference images",
        },
    );
}

#[test]
fn interpolating_between_two_stills_fixes_the_length_too() {
    let p = asking(|r| {
        r.resolution = VideoResolution::P720;
        r.seconds = ClipSeconds::Four;
        r.first_image = Some(asset_id("logo"));
        r.last_image = Some(asset_id("logo"));
    });
    assert_only_problem(
        &p,
        V::LengthLocked {
            asset: asset_id("shot-city"),
            asked: 4,
            cause: "a first and last image",
        },
    );
}
