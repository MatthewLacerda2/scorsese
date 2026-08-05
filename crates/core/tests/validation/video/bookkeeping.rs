//! Records on kinds that could never have produced them — and the rule that
//! a brief with several problems reports all of them.

use super::asking;
use crate::common::{assert_only_problem, asset_id, asset_mut, project};
use scorsese_core::{
    AssetField as F, AssetKind, AssetProblem as E, ClipSeconds, VideoModel, VideoRequest,
};

#[test]
fn a_request_on_a_kind_that_takes_none_is_refused() {
    let mut p = project();
    asset_mut(&mut p, "vo-open").video = Some(VideoRequest::default());
    assert_only_problem(
        &p,
        E::StrayField {
            asset: asset_id("vo-open"),
            field: F::Video,
            kind: AssetKind::GeneratedAudio,
        },
    );
}

#[test]
fn a_price_on_something_free_is_refused() {
    let mut p = project();
    asset_mut(&mut p, "score").estimated_cost_cents = Some(96);
    assert_only_problem(
        &p,
        E::StrayField {
            asset: asset_id("score"),
            field: F::Cost,
            kind: AssetKind::SynthAudio,
        },
    );
}

#[test]
fn a_ticket_on_an_imported_file_names_work_nobody_is_doing() {
    let mut p = project();
    asset_mut(&mut p, "logo").operation = Some("operations/abc".to_owned());
    assert_only_problem(
        &p,
        E::StrayField {
            asset: asset_id("logo"),
            field: F::Operation,
            kind: AssetKind::Image,
        },
    );
}

#[test]
fn every_conflict_in_one_request_is_reported_at_once() {
    let p = asking(|r| {
        r.model = VideoModel::Lite;
        r.seconds = ClipSeconds::Four;
        r.reference_images = vec![asset_id("logo"); 4];
    });
    assert_eq!(
        p.validate().expect_err("three things are wrong").len(),
        3,
        "a brief with three problems reports three, not the first"
    );
}
