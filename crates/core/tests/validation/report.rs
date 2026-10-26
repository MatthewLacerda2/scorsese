//! How a set of problems reads line by line, for a caller that lays the list
//! out itself instead of printing the whole report.

use crate::common::{asset_id, project};
use scorsese_core::{Asset, Curve, Geometry, Heads, Point, Rgba, Shape};

/// A project with two problems: an arrow that is filled and has no stroke.
fn twice_wrong() -> scorsese_core::Project {
    let arrow = Geometry::Arrow {
        from: Point::new(0.2, 0.3).into(),
        to: Point::new(0.8, 0.7).into(),
        curve: Curve::Straight,
        heads: Heads::End,
    };
    let mut p = project();
    p.assets.push(Asset::shape(
        asset_id("a-to-b"),
        Shape::filled(arrow, Rgba::WHITE),
    ));
    p
}

#[test]
fn messages_are_each_problem_in_its_own_words_in_order() {
    let errors = twice_wrong().validate().expect_err("two problems");
    let each: Vec<String> = errors.as_slice().iter().map(ToString::to_string).collect();
    assert_eq!(each.len(), 2, "{each:?}");
    assert_eq!(errors.messages(), each);
}

#[test]
fn the_first_message_is_the_first_problem() {
    let errors = twice_wrong().validate().expect_err("two problems");
    let first = errors.as_slice()[0].to_string();
    assert_eq!(errors.first_message(), Some(first));
}

#[test]
fn the_report_counts_its_problems_and_hands_them_over() {
    let errors = twice_wrong().validate().expect_err("two problems");
    assert!(!errors.is_empty());
    assert!(
        errors
            .to_string()
            .starts_with("2 problems in this project:\n  - "),
        "{errors}"
    );
    let each = errors.messages();
    let taken: Vec<String> = errors.into_iter().map(|e| e.to_string()).collect();
    assert_eq!(taken, each);

    let mut once = project();
    crate::common::asset_mut(&mut once, "logo").path = None;
    let one = once.validate().expect_err("one problem");
    assert!(
        one.to_string().starts_with("1 problem in this project:"),
        "{one}"
    );
}
