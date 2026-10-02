//! What a generated still asks for, and what the vendor will not draw.

use crate::common::{assert_only_problem, asset_id, problems, project};
use scorsese_core::{
    Asset, AssetField, AssetKind, AssetProblem, ImageModel, ImageProblem as I, ImageRequest,
    ImageResolution, Project,
};

/// The fixture project with a sketched still in it, its request edited.
fn drawing(edit: impl FnOnce(&mut ImageRequest)) -> Project {
    let mut p = project();
    let mut still = Asset::sketch(asset_id("poster"), AssetKind::GeneratedImage, "a poster");
    let mut request = ImageRequest::default();
    edit(&mut request);
    still.image = Some(request);
    p.assets.push(still);
    p
}

#[test]
fn a_sentence_alone_is_a_whole_request() {
    assert_eq!(problems(&drawing(|_| {})), vec![]);
}

#[test]
fn the_cheaper_model_draws_one_size_and_says_so() {
    let p = drawing(|r| {
        r.model = ImageModel::Lite;
        r.resolution = Some(ImageResolution::K4);
    });
    assert_only_problem(
        &p,
        I::ResolutionUnsupported {
            asset: asset_id("poster"),
            model: "lite",
            resolution: "4K",
        },
    );
}

#[test]
fn the_cheaper_model_with_no_size_named_takes_the_one_it_draws() {
    let p = drawing(|r| r.model = ImageModel::Lite);
    assert_eq!(problems(&p), vec![]);
}

#[test]
fn a_still_and_a_generated_still_are_both_references() {
    let p = drawing(|r| r.reference_images = vec![asset_id("logo")]);
    assert_eq!(problems(&p), vec![]);
    let mut p = drawing(|r| r.reference_images = vec![asset_id("sheet")]);
    p.assets.push(Asset::sketch(
        asset_id("sheet"),
        AssetKind::GeneratedImage,
        "a character sheet",
    ));
    assert_eq!(problems(&p), vec![]);
}

#[test]
fn a_reference_that_is_not_a_picture_is_reported() {
    let p = drawing(|r| r.reference_images = vec![asset_id("score")]);
    assert_only_problem(
        &p,
        I::NotAnImage {
            asset: asset_id("poster"),
            referenced: asset_id("score"),
            kind: AssetKind::SynthAudio,
        },
    );
}

#[test]
fn a_reference_that_names_nothing_or_itself_is_reported() {
    let p = drawing(|r| r.reference_images = vec![asset_id("gone")]);
    assert_only_problem(
        &p,
        I::UnknownImage {
            asset: asset_id("poster"),
            referenced: asset_id("gone"),
        },
    );
    let p = drawing(|r| r.reference_images = vec![asset_id("poster")]);
    assert_only_problem(
        &p,
        I::ReferencesItself {
            asset: asset_id("poster"),
        },
    );
}

#[test]
fn a_fifteenth_reference_is_one_too_many() {
    let p = drawing(|r| r.reference_images = vec![asset_id("logo"); 15]);
    assert_only_problem(
        &p,
        I::TooManyReferenceImages {
            asset: asset_id("poster"),
            found: 15,
            max: 14,
        },
    );
}

#[test]
fn an_image_request_on_another_kind_is_refused() {
    let mut p = project();
    let shot = p
        .assets
        .iter_mut()
        .find(|a| a.id == asset_id("shot-city"))
        .expect("the fixture's shot");
    shot.image = Some(ImageRequest::default());
    assert_only_problem(
        &p,
        AssetProblem::StrayField {
            asset: asset_id("shot-city"),
            field: AssetField::Image,
            kind: AssetKind::GeneratedVideo,
        },
    );
}
