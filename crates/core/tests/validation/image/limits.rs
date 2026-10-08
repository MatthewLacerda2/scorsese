//! What each model takes, as Google's page documents it: sizes, shapes,
//! references per kind, and thinking levels.

use super::drawing;
use crate::common::{assert_only_problem, asset_id, problems};
use scorsese_core::{
    ImageAspect, ImageModel, ImageProblem as I, ImageResolution, ImageThinking, ReferenceKind,
};

#[test]
fn the_default_is_nano_banana_2_1_at_2k() {
    let p = drawing(|_| {});
    let request = p.assets.last().expect("the still").image_request();
    assert_eq!(request.model, ImageModel::NanoBanana21);
    assert_eq!(request.size(), ImageResolution::K2);
    assert_eq!(request.thinking(), Some(ImageThinking::Medium));
}

#[test]
fn half_k_is_flash_s_alone_and_says_what_else_is_drawn() {
    for model in [ImageModel::NanoBanana21, ImageModel::Pro] {
        let p = drawing(|r| {
            r.model = model;
            r.resolution = Some(ImageResolution::K05);
        });
        assert_only_problem(
            &p,
            I::ResolutionUnsupported {
                asset: asset_id("poster"),
                model: model.as_str(),
                resolution: "0.5K",
                draws: String::from("1K, 2K or 4K"),
            },
        );
    }
    let p = drawing(|r| {
        r.model = ImageModel::Flash;
        r.resolution = Some(ImageResolution::K05);
    });
    assert_eq!(problems(&p), vec![]);
}

#[test]
fn the_long_strips_are_drawn_by_2_1_and_flash_only() {
    for (model, drawn) in [
        (ImageModel::NanoBanana21, true),
        (ImageModel::Flash, true),
        (ImageModel::Lite, false),
        (ImageModel::Pro, false),
    ] {
        let p = drawing(|r| {
            r.model = model;
            r.aspect = ImageAspect::EightOne;
        });
        if drawn {
            assert_eq!(problems(&p), vec![], "{model:?}");
        } else {
            assert_only_problem(
                &p,
                I::AspectUnsupported {
                    asset: asset_id("poster"),
                    model: model.as_str(),
                    aspect: "8:1",
                },
            );
        }
    }
}

#[test]
fn each_kind_of_reference_is_capped_by_its_model_and_refused_with_the_number() {
    for model in ImageModel::ALL {
        for kind in ReferenceKind::ALL {
            let max = model.references(kind);
            let p = drawing(|r| {
                r.model = model;
                *r.references_of_mut(kind) = vec![asset_id("logo"); max];
            });
            assert_eq!(problems(&p), vec![], "{model:?} takes {max} {kind:?}");
            let p = drawing(|r| {
                r.model = model;
                *r.references_of_mut(kind) = vec![asset_id("logo"); max + 1];
            });
            assert_only_problem(
                &p,
                I::TooManyReferenceImages {
                    asset: asset_id("poster"),
                    model: model.as_str(),
                    field: kind.field(),
                    found: max + 1,
                    max,
                },
            );
        }
    }
}

#[test]
fn the_documented_limits_are_the_ones_answered() {
    let limits = |model: ImageModel| ReferenceKind::ALL.map(|kind| model.references(kind));
    assert_eq!(limits(ImageModel::NanoBanana21), [10, 4, 0]);
    assert_eq!(limits(ImageModel::Flash), [10, 4, 0]);
    assert_eq!(limits(ImageModel::Lite), [14, 0, 0]);
    assert_eq!(limits(ImageModel::Pro), [6, 5, 3]);
}

#[test]
fn a_thinking_level_the_model_does_not_offer_is_refused() {
    let p = drawing(|r| {
        r.model = ImageModel::Flash;
        r.thinking = Some(ImageThinking::Medium);
    });
    assert_only_problem(
        &p,
        I::ThinkingUnsupported {
            asset: asset_id("poster"),
            model: "flash",
            thinking: "medium",
            offers: String::from("minimal or high"),
        },
    );
    let p = drawing(|r| {
        r.model = ImageModel::Pro;
        r.thinking = Some(ImageThinking::High);
    });
    assert_only_problem(
        &p,
        I::ThinkingUnsupported {
            asset: asset_id("poster"),
            model: "pro",
            thinking: "high",
            offers: String::from("no level to choose"),
        },
    );
    let p = drawing(|r| r.thinking = Some(ImageThinking::High));
    assert_eq!(problems(&p), vec![]);
}

#[test]
fn every_kind_of_reference_must_name_a_picture() {
    let p = drawing(|r| {
        r.model = ImageModel::Pro;
        r.character_images = vec![asset_id("gone")];
        r.style_images = vec![asset_id("score")];
    });
    let found = problems(&p);
    assert_eq!(found.len(), 2, "{found:?}");
}
