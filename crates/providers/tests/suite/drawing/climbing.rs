//! A still named for an id that climbs out of the project is refused before it
//! is drawn: refused while refusing still costs nothing.

use scorsese_core::AssetId;
use scorsese_providers::credentials::Budget;
use scorsese_providers::image::{Outcome, batch, generate};

use crate::drawing::mock::Mock;
use crate::drawing::sketched;

#[test]
fn a_still_whose_id_climbs_is_never_drawn() {
    let (dir, mut project, _) = sketched("climbs", "a lighthouse at dusk");
    project.assets[0].id = AssetId::new("../../escaped");
    let provider = Mock::willing();

    let run = generate(&mut project, &dir, &provider, Budget::unlimited(0));

    assert_eq!(provider.requests(), 0, "nothing was paid for: {run:?}");
    std::fs::remove_dir_all(&dir).ok();
}

/// A batched still is paid for when it is ordered and lands when it is
/// collected, under a key made of its id; an id validation refuses, a `/` in
/// it, could never land. So it is never ordered: the batch pays for nothing it
/// would then drop.
#[test]
fn a_batched_still_whose_id_validation_refuses_is_never_ordered() {
    let (dir, mut project, _) = sketched("batch-slash", "a lighthouse at dusk");
    project.assets[0].id = AssetId::new("posters/one");
    assert!(project.validate().is_err(), "the rule refuses the id");
    let provider = Mock::willing();

    let run = batch(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    assert!(
        provider.ordered.borrow().is_empty(),
        "nothing ordered: {run:?}"
    );
    assert_eq!(provider.requests(), 0);
    assert!(
        matches!(run[0].1, Outcome::Incomplete { .. }),
        "and it says why: {run:?}"
    );
    std::fs::remove_dir_all(&dir).ok();
}
