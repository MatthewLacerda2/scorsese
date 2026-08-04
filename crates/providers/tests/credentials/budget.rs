//! The ceiling, and the thing it will not let a flag do.

use scorsese_providers::credentials::{Budget, Settings};

#[test]
fn a_run_inside_the_ceiling_goes_ahead() {
    assert!(Budget::new(5000, 3800).check(400).is_ok());
}

#[test]
fn a_run_landing_exactly_on_the_ceiling_goes_ahead() {
    assert!(Budget::new(5000, 4600).check(400).is_ok());
}

#[test]
fn a_run_over_the_ceiling_says_by_how_much() {
    let refused = Budget::new(5000, 4800)
        .check(400)
        .expect_err("that is 200 over");
    assert_eq!(refused.over, 200);
    assert_eq!(refused.ceiling, 5000);
    assert_eq!(refused.spent, 4800);
}

/// A fresh install has no ceiling, and inventing one would refuse the first
/// honest run somebody made.
#[test]
fn no_ceiling_refuses_nothing() {
    assert!(Budget::unlimited(100_000).check(50_000).is_ok());
    assert_eq!(Budget::unlimited(0).remaining(), None);
}

#[test]
fn what_is_left_is_what_the_ceiling_leaves() {
    assert_eq!(Budget::new(5000, 3800).remaining(), Some(1200));
    assert_eq!(
        Budget::new(5000, 9000).remaining(),
        Some(0),
        "already over is nothing left, never a negative"
    );
}

#[test]
fn the_ceiling_comes_from_the_settings_file() {
    let settings = Settings {
        budget_cents: Some(1200),
        ..Settings::default()
    };
    assert_eq!(Budget::from_settings(&settings, 0).remaining(), Some(1200));
    assert_eq!(
        Budget::from_settings(&Settings::default(), 0).remaining(),
        None
    );
}
