//! A **shipped** family asked for a cut it was never drawn in — an italic it
//! does not have, or a weight off its list or its axis.
//!
//! A problem, not a warning: the render refuses each of these rather than
//! reaching for a neighbouring cut, so `check` saying nothing about them would
//! be a clean bill of health for a project that cannot render.

use crate::common::documents::{clip, project};
use crate::common::run_in;

/// A title in a shipped face with the given extra style fields.
fn styled(face: &str, extra: &str) -> String {
    format!(
        r#"{{ "id": "title", "kind": "text", "text": "Hello",
              "style": {{ "font": "{face}"{extra} }} }}"#
    )
}

fn checked(label: &str, asset: String) -> crate::common::Run {
    let dir = project(label, &[asset], &[clip("c1", "title", 0)]);
    run_in(&dir, &["check"])
}

#[test]
fn an_italic_the_family_never_drew_is_a_problem() {
    let run = checked("face-no-italic", styled("anton", r#", "italic": true"#));
    assert!(run.failed, "the render refuses this:\n{}", run.output);
    run.says("asset `title`");
    run.says("`anton` cannot be set as asked");
    run.says("has no italic");
}

#[test]
fn a_weight_the_family_was_not_drawn_at_is_a_problem() {
    let run = checked(
        "face-not-drawn",
        styled("liberation-sans", r#", "weight": 600"#),
    );
    assert!(run.failed, "the render refuses this:\n{}", run.output);
    run.says("`liberation-sans` cannot be set as asked");
    run.says("no weight 600");
}

#[test]
fn a_weight_off_the_axis_is_a_problem() {
    let run = checked("face-off-axis", styled("inter", r#", "weight": 50"#));
    assert!(run.failed, "the render refuses this:\n{}", run.output);
    run.says("`inter` cannot be set as asked");
    run.says("no weight 50");
}

#[test]
fn a_cut_the_family_has_says_nothing() {
    let run = checked(
        "face-fine",
        styled("inter", r#", "italic": true, "weight": 700"#),
    );
    assert!(!run.failed, "{}", run.output);
    run.silent_about("cannot be set as asked");
    run.says("no problems, nothing to warn about");
}

#[test]
fn a_name_nothing_ships_is_one_finding_not_two() {
    let run = checked("face-unknown", styled("comic-sans", r#", "italic": true"#));
    assert!(run.failed, "{}", run.output);
    run.says("there is no font called `comic-sans`");
    run.silent_about("cannot be set as asked");
}
