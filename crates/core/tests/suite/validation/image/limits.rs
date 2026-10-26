//! What a still takes, as Google's page documents Nano Banana 2.1: every size
//! and shape it offers, references per kind, and thinking levels.

use super::drawing;
use crate::common::{assert_only_problem, asset_id, problems};
use scorsese_core::{
    ImageAspect, ImageProblem as I, ImageRequest, ImageResolution, ImageThinking, ReferenceKind,
};

#[test]
fn the_default_is_2k_thinking_at_medium() {
    let p = drawing(|_| {});
    let request = p.assets.last().expect("the still").image_request();
    assert_eq!(request.size(), ImageResolution::K2);
    assert_eq!(request.thinking(), ImageThinking::Medium);
}

#[test]
fn every_size_shape_and_thinking_level_is_drawn() {
    for resolution in ImageResolution::ALL {
        for aspect in ImageAspect::ALL {
            for thinking in ImageThinking::ALL {
                let p = drawing(|r| {
                    r.resolution = Some(resolution);
                    r.aspect = aspect;
                    r.thinking = Some(thinking);
                });
                assert_eq!(
                    problems(&p),
                    vec![],
                    "{resolution:?} {aspect:?} {thinking:?}"
                );
            }
        }
    }
}

#[test]
fn each_kind_of_reference_is_capped_and_refused_with_the_number() {
    for kind in ReferenceKind::ALL {
        let max = kind.max();
        let p = drawing(|r| *r.references_of_mut(kind) = vec![asset_id("logo"); max]);
        assert_eq!(problems(&p), vec![], "a still takes {max} {kind:?}");
        let p = drawing(|r| *r.references_of_mut(kind) = vec![asset_id("logo"); max + 1]);
        assert_only_problem(
            &p,
            I::TooManyReferenceImages {
                asset: asset_id("poster"),
                field: kind.field(),
                found: max + 1,
                max,
            },
        );
    }
}

#[test]
fn the_documented_limits_are_the_ones_answered() {
    assert_eq!(ReferenceKind::ALL.map(ReferenceKind::max), [10, 4]);
}

#[test]
fn every_kind_of_reference_must_name_a_picture() {
    let p = drawing(|r| {
        r.reference_images = vec![asset_id("score")];
        r.character_images = vec![asset_id("gone")];
    });
    let found = problems(&p);
    assert_eq!(found.len(), 2, "{found:?}");
}

/// A refusal names the field to edit, so the name it gives is the one
/// `project.json` spells.
#[test]
fn each_kind_names_the_field_that_holds_it() {
    for kind in ReferenceKind::ALL {
        let mut request = ImageRequest::default();
        request.references_of_mut(kind).push(asset_id("logo"));
        let written = serde_json::to_value(&request).expect("serialises");
        assert!(written.get(kind.field()).is_some(), "{kind:?}: {written}");
    }
}

/// What only another model took is not in the format any more (#1308): a
/// block naming a model, a style or the half-K size is refused as written.
#[test]
fn a_model_a_style_and_half_k_are_not_in_the_format() {
    for block in [
        r#"{ "model": "nano_banana_2.1" }"#,
        r#"{ "style_images": ["logo"] }"#,
        r#"{ "resolution": "0.5K" }"#,
    ] {
        assert!(
            serde_json::from_str::<ImageRequest>(block).is_err(),
            "{block} is refused"
        );
    }
}
