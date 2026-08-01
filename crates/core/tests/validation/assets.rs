//! Assets-table coherence: identity, hashes, and what each kind must carry.

use crate::common::{assert_only_problem, asset_id, asset_mut, problems, project};
use scorsese_core::{AssetField as F, AssetKind, AssetProblem as E, GenerationState};

#[test]
fn a_reused_asset_id_is_reported_once() {
    let mut p = project();
    let duplicate = p.assets[1].clone();
    p.assets.push(duplicate);
    assert_only_problem(
        &p,
        E::DuplicateAssetId {
            id: asset_id("logo"),
        },
    );
}

#[test]
fn a_file_backed_asset_needs_a_path() {
    let mut p = project();
    asset_mut(&mut p, "logo").path = None;
    assert_only_problem(
        &p,
        E::MissingField {
            asset: asset_id("logo"),
            field: F::Path,
            kind: AssetKind::Image,
        },
    );
}

#[test]
fn a_malformed_hash_is_refused() {
    let mut p = project();
    asset_mut(&mut p, "logo").sha256 = Some("NOTAHASH".to_owned());
    assert_only_problem(
        &p,
        E::BadSha256 {
            asset: asset_id("logo"),
            value: "NOTAHASH".to_owned(),
        },
    );
}

#[test]
fn an_uppercase_hash_is_refused() {
    let mut p = project();
    let shouty = "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855";
    asset_mut(&mut p, "logo").sha256 = Some(shouty.to_owned());
    assert_only_problem(
        &p,
        E::BadSha256 {
            asset: asset_id("logo"),
            value: shouty.to_owned(),
        },
    );
}

#[test]
fn a_prompt_asset_needs_a_prompt_and_a_state() {
    let mut p = project();
    let shot = asset_mut(&mut p, "shot-city");
    shot.prompt = None;
    shot.state = None;
    let kind = AssetKind::GeneratedVideo;
    let found = problems(&p);
    assert!(
        found.contains(
            &E::MissingField {
                asset: asset_id("shot-city"),
                field: F::Prompt,
                kind
            }
            .into()
        )
    );
    assert!(
        found.contains(
            &E::MissingField {
                asset: asset_id("shot-city"),
                field: F::State,
                kind
            }
            .into()
        )
    );
}

#[test]
fn a_generated_asset_must_point_at_its_file() {
    let mut p = project();
    asset_mut(&mut p, "shot-city").state = Some(GenerationState::Generated);
    assert_only_problem(
        &p,
        E::GeneratedWithoutPath {
            asset: asset_id("shot-city"),
        },
    );
}

#[test]
fn a_sketch_asset_needs_no_file_yet() {
    let p = project();
    let shot = p.asset(&asset_id("shot-city")).expect("asset");
    assert!(shot.needs_generation());
    assert!(
        !shot.has_renderable_media(),
        "a sketch renders as a slug card"
    );
    assert_eq!(p.validate(), Ok(()));
}

#[test]
fn a_plain_asset_carries_no_prompt_or_state() {
    let mut p = project();
    let logo = asset_mut(&mut p, "logo");
    logo.prompt = Some("a logo, but nicer".to_owned());
    logo.state = Some(GenerationState::Sketch);
    let kind = AssetKind::Image;
    let found = problems(&p);
    assert!(
        found.contains(
            &E::StrayField {
                asset: asset_id("logo"),
                field: F::Prompt,
                kind
            }
            .into()
        )
    );
    assert!(
        found.contains(
            &E::StrayField {
                asset: asset_id("logo"),
                field: F::State,
                kind
            }
            .into()
        )
    );
}
