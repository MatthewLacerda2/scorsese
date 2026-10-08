//! Ordering: every sketch in as few jobs as the vendor allows, each ticket
//! written down, the ceiling counted across them.

use scorsese_core::{Asset, AssetId, AssetKind, GenerationState, ImageModel, ImageRequest};
use scorsese_providers::credentials::Budget;
use scorsese_providers::image::{Outcome, batch, generate};

use super::half;
use crate::mock::Mock;
use crate::sketched;

#[test]
fn every_sketch_is_ordered_in_one_job_and_its_ticket_written_down() {
    let (dir, mut project, id) = sketched("batch-order", "a lighthouse at dusk");
    project.assets.push(Asset::sketch(
        AssetId::new("second"),
        AssetKind::GeneratedImage,
        "a harbour",
    ));
    let provider = Mock::willing();

    let run = batch(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert_eq!(provider.ordered.borrow().len(), 1, "one model, one job");
    assert_eq!(provider.ordered.borrow()[0].len(), 2);
    assert_eq!(provider.requests(), 0, "nothing was drawn now");
    let cents = half(&project, &dir, &id);
    assert_eq!(
        run[0].1,
        Outcome::Ordered {
            operation: String::from("batches/1"),
            estimated_cost_cents: cents,
        }
    );
    assert_eq!(run[0].1.spent_cents(), cents, "ordering is the spending");
    let asset = project.asset(&id).expect("the asset");
    assert_eq!(asset.state, Some(GenerationState::Queued));
    assert_eq!(asset.operation.as_deref(), Some("batches/1"));
    assert!(asset.queued_at.is_some());
    project
        .validate()
        .expect("a still in a batch is a valid document");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn stills_on_two_models_are_two_jobs() {
    let (dir, mut project, _) = sketched("batch-models", "a lighthouse");
    let mut pro = Asset::sketch(AssetId::new("pro"), AssetKind::GeneratedImage, "a harbour");
    pro.image = Some(ImageRequest {
        model: ImageModel::Pro,
        ..ImageRequest::default()
    });
    project.assets.push(pro);
    let provider = Mock::willing();

    batch(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert_eq!(provider.ordered.borrow().len(), 2);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_batch_over_the_ceiling_orders_nothing() {
    let (dir, mut project, id) = sketched("batch-ceiling", "a lighthouse");
    let provider = Mock::willing();

    let refused = batch(&mut project, &dir, &provider, Budget::new(1, 1));

    assert!(refused.is_err());
    assert!(provider.ordered.borrow().is_empty());
    assert_eq!(project.asset(&id).expect("there").operation, None);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_still_already_drawn_is_recorded_and_never_ordered() {
    let (dir, mut project, _) = sketched("batch-cached", "a lighthouse");
    let provider = Mock::willing();
    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("drawn now");

    let run = batch(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert!(matches!(run[0].1, Outcome::Cached { .. }), "{:?}", run[0].1);
    assert!(provider.ordered.borrow().is_empty());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_ceiling_counts_every_job_the_run_has_ordered() {
    let (dir, mut project, _) = sketched("batch-ceiling-jobs", "a lighthouse");
    let mut pro = Asset::sketch(AssetId::new("pro"), AssetKind::GeneratedImage, "a harbour");
    pro.image = Some(ImageRequest {
        model: ImageModel::Pro,
        ..ImageRequest::default()
    });
    project.assets.push(pro);
    let provider = Mock::willing();

    let refused = batch(&mut project, &dir, &provider, Budget::new(9, 0));

    assert!(refused.is_err());
    assert_eq!(provider.ordered.borrow().len(), 1, "the first job fit");
    std::fs::remove_dir_all(&dir).ok();
}
