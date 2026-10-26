//! A still ordered in a batch is spent the moment it is ordered (#1107), so a
//! batch that does not come good the way it was bought still counts.

use scorsese_providers::credentials::Budget;
use scorsese_providers::image::{Outcome, batch, generate};
use scorsese_providers::spending::so_far;

use super::half;
use crate::drawing::mock::Mock;
use crate::drawing::{asset_mut, sketched};

/// Its picture was drawn from the old brief and kept in `generated/`: paid
/// for, whatever the asset asks for now.
#[test]
fn a_still_whose_brief_changed_while_it_waited_is_still_spent() {
    let (dir, mut project, id) = sketched("batch-edit-spent", "a lighthouse");
    let cents = half(&project, &dir, &id);
    let provider = Mock::willing();
    batch(&mut project, &dir, &provider, Budget::unlimited(0)).expect("ordered");
    asset_mut(&mut project, &id).prompt = Some(String::from("a windmill"));
    *provider.finished.borrow_mut() = Some(Ok(()));

    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert!(matches!(&run[0].1, Outcome::Failed { .. }), "{run:?}");
    let spending = so_far(&project, &dir);
    assert_eq!(spending.generation_cents, cents);
    assert_eq!(spending.superseded.takes, 1, "no asset shows it");
    std::fs::remove_dir_all(&dir).ok();
}

/// Whether Gemini bills a batch it stops is not documented, so a ceiling
/// counts it: the safe side of an unknown is the one that spends less next.
#[test]
fn a_batch_that_stopped_is_still_spent() {
    let (dir, mut project, id) = sketched("batch-stop-spent", "a lighthouse");
    let cents = half(&project, &dir, &id);
    let provider = Mock::willing();
    batch(&mut project, &dir, &provider, Budget::unlimited(0)).expect("ordered");
    *provider.finished.borrow_mut() = Some(Err(String::from("the batch expired")));

    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert_eq!(so_far(&project, &dir).generation_cents, cents);
    std::fs::remove_dir_all(&dir).ok();
}
