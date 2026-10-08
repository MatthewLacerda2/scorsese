//! Half price within a day (#894): ordered, waited on, collected — and never
//! ordered or drawn twice.

use scorsese_core::{Asset, AssetId, AssetKind, GenerationState, ImageModel, ImageRequest};
use scorsese_providers::credentials::Budget;
use scorsese_providers::image::{Brief, Outcome, batch, generate};
use scorsese_providers::prices;

use crate::mock::Mock;
use crate::{asset_mut, sketched};

/// What the still `id` costs in a batch, by the one function that prices it.
fn half(project: &scorsese_core::Project, dir: &std::path::Path, id: &AssetId) -> u64 {
    let brief = Brief::of(project, dir, project.asset(id).expect("there")).expect("whole");
    prices::image_in_batch(&brief.request, brief.characters(), 0)
        .expect("priced")
        .cents
}

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
fn a_still_in_a_batch_is_asked_after_and_never_ordered_or_drawn_again() {
    let (dir, mut project, _) = sketched("batch-wait", "a lighthouse");
    let provider = Mock::willing();
    batch(&mut project, &dir, &provider, Budget::unlimited(0)).expect("ordered");

    let again = batch(&mut project, &dir, &provider, Budget::unlimited(0)).expect("again");
    let now = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("now");

    let waiting = Outcome::Waiting {
        operation: String::from("batches/1"),
    };
    assert_eq!(again[0].1, waiting);
    assert_eq!(now[0].1, waiting);
    assert_eq!(provider.ordered.borrow().len(), 1);
    assert_eq!(provider.requests(), 0);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_finished_batch_lands_at_half_price_on_the_next_run() {
    let (dir, mut project, id) = sketched("batch-land", "a lighthouse");
    let provider = Mock::willing();
    batch(&mut project, &dir, &provider, Budget::unlimited(0)).expect("ordered");
    *provider.finished.borrow_mut() = Some(Ok(()));

    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    let Outcome::Collected { path, .. } = &run[0].1 else {
        panic!("{:?}", run[0].1);
    };
    assert_eq!(run[0].1.spent_cents(), 0, "the ordering run spent it");
    assert!(run[0].1.landed());
    assert!(path.resolve(&dir).is_file());
    let cents = half(&project, &dir, &id);
    let asset = project.asset(&id).expect("the asset");
    assert_eq!(asset.state, Some(GenerationState::Generated));
    assert_eq!(asset.operation, None);
    assert_eq!(asset.estimated_cost_cents, Some(cents));
    assert_eq!(provider.requests(), 0, "collected, not drawn");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_batch_that_stopped_puts_the_still_back_and_the_run_does_not_draw_it() {
    let (dir, mut project, id) = sketched("batch-stop", "a lighthouse");
    let provider = Mock::willing();
    batch(&mut project, &dir, &provider, Budget::unlimited(0)).expect("ordered");
    *provider.finished.borrow_mut() = Some(Err(String::from("the batch expired")));

    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert_eq!(
        run,
        vec![(
            id.clone(),
            Outcome::Failed {
                message: String::from("the batch expired")
            }
        )]
    );
    let asset = project.asset(&id).expect("the asset");
    assert_eq!(asset.state, Some(GenerationState::Sketch));
    assert_eq!(asset.operation, None);
    assert_eq!(
        provider.requests(),
        0,
        "this run's quote priced it at nothing"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_still_edited_while_it_waited_keeps_the_old_picture_and_goes_back_to_a_sketch() {
    let (dir, mut project, id) = sketched("batch-edit", "a lighthouse");
    let provider = Mock::willing();
    batch(&mut project, &dir, &provider, Budget::unlimited(0)).expect("ordered");
    let old = provider.ordered.borrow()[0][0].clone();
    asset_mut(&mut project, &id).prompt = Some(String::from("a windmill"));
    *provider.finished.borrow_mut() = Some(Ok(()));

    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert!(matches!(&run[0].1, Outcome::Failed { message } if message.contains("changed")));
    assert!(
        dir.join(format!("generated/{old}.jpg")).is_file(),
        "paid for, kept"
    );
    assert_eq!(project.asset(&id).expect("there").operation, None);
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
