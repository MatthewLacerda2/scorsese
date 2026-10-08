//! Collecting: a batch asked after, landed at half price, or given back.

use scorsese_core::GenerationState;
use scorsese_providers::credentials::Budget;
use scorsese_providers::image::{Outcome, batch, generate};

use super::half;
use crate::mock::Mock;
use crate::{asset_mut, sketched};

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
