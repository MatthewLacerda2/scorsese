//! A shot past the window its video is kept in was paid for and never arrived
//! (#1107): released so it can be asked for again, and still counted.

use scorsese_core::Timestamp;
use scorsese_providers::credentials::Budget;
use scorsese_providers::spending::so_far;
use scorsese_providers::video::{Outcome, generate};

use crate::generating::mock::Mock;
use crate::generating::sketched;

#[test]
fn an_expired_shot_is_still_spent_and_a_ceiling_counts_it() {
    let (dir, mut project, id) = sketched("expired-spent", "a desert road");
    let provider = Mock::default();
    let sent = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("sent");
    let cents = sent[0].1.spent_cents();
    project.asset_mut(&id).expect("the asset").queued_at =
        Timestamp::from_utc(2020, 1, 1, 0, 0, 0).ok();

    let polled = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("polled");
    assert!(matches!(polled[0].1, Outcome::Expired { .. }), "{polled:?}");

    let spent = so_far(&project, &dir);
    assert_eq!(spent.generation_cents, cents, "the money is gone");
    assert_eq!(spent.superseded.takes, 1, "no asset shows the take");
    // A ceiling of one shot has no room for it again.
    let budget = Budget::new(cents, spent.total());
    assert!(generate(&mut project, &dir, &provider, budget).is_err());
    assert_eq!(provider.submissions(), 1, "nothing sent past the ceiling");
    std::fs::remove_dir_all(&dir).ok();
}
