//! The pictures a request is built from: how many, which tier takes them,
//! and whether the ids point at pictures at all.

use super::asking;
use crate::common::{assert_only_problem, asset_id};
use scorsese_core::{AssetId, AssetKind, VideoModel, VideoProblem as V};

#[test]
fn the_cheaper_tier_refuses_reference_images_rather_than_dropping_them() {
    let p = asking(|r| {
        r.model = VideoModel::Lite;
        r.reference_images = vec![asset_id("logo")];
    });
    assert_only_problem(
        &p,
        V::ReferenceImagesUnsupported {
            asset: asset_id("shot-city"),
            model: "lite",
        },
    );
}

#[test]
fn a_fourth_reference_image_is_one_too_many() {
    let p = asking(|r| r.reference_images = vec![asset_id("logo"); 4]);
    assert_only_problem(
        &p,
        V::TooManyReferenceImages {
            asset: asset_id("shot-city"),
            found: 4,
            max: 3,
        },
    );
}

#[test]
fn a_last_still_needs_the_one_it_starts_from() {
    let p = asking(|r| r.last_image = Some(asset_id("logo")));
    assert_only_problem(
        &p,
        V::LastImageWithoutFirst {
            asset: asset_id("shot-city"),
        },
    );
}

#[test]
fn a_still_that_names_nothing_is_reported() {
    let p = asking(|r| r.first_image = Some(AssetId::new("gone")));
    assert_only_problem(
        &p,
        V::UnknownImage {
            asset: asset_id("shot-city"),
            referenced: AssetId::new("gone"),
        },
    );
}

#[test]
fn a_still_that_is_not_a_picture_is_reported() {
    let p = asking(|r| r.first_image = Some(asset_id("score")));
    assert_only_problem(
        &p,
        V::NotAnImage {
            asset: asset_id("shot-city"),
            referenced: asset_id("score"),
            kind: AssetKind::SynthAudio,
        },
    );
}
