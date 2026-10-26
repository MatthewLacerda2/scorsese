//! A symbol this build does not ship.
//!
//! A **problem**, and the same one an unknown font is: nothing can draw it, so
//! the layer comes out empty — and an empty layer is invisible to anything
//! counting frames. The catalogue is far too large to list in a message, so
//! what makes this refusal usable at all is the near matches beside it.

use crate::common::documents::{clip, project};
use crate::common::run_in;

/// An icon asset naming a symbol. Written out here rather than through a
/// helper because the name is the whole subject.
fn named(id: &str, name: &str) -> String {
    format!(
        r##"{{ "id": "{id}", "kind": "icon",
               "icon": {{ "name": "{name}", "size": 0.12, "color": "#ffffffff" }} }}"##
    )
}

fn checked(label: &str, name: &str) -> crate::common::Run {
    let dir = project(label, &[named("badge", name)], &[clip("c1", "badge", 0)]);
    run_in(&dir, &["check"])
}

#[test]
fn a_symbol_this_build_ships_is_no_problem_at_all() {
    let run = checked("icon-known", "clapperboard");
    assert!(!run.failed, "{}", run.output);
    run.says("no problems, nothing to warn about");
}

#[test]
fn a_name_nothing_ships_is_a_problem_and_fails() {
    let run = checked("icon-unknown", "clapperbord");
    assert!(
        run.failed,
        "an icon nothing can draw stops a render:\n{}",
        run.output
    );
    run.says("problem: asset `badge`");
    run.says("there is no icon called `clapperbord`");
}

/// The half that makes the refusal actionable — and the reason it is not simply
/// "that is not an icon".
#[test]
fn the_refusal_names_what_to_write_instead() {
    checked("icon-typo", "clapperbord").says("Did you mean `clapperboard`");
    // The half-remembered compound, which an edit distance alone never finds.
    checked("icon-compound", "clapper").says("`clapperboard`");
}

/// A name upstream retired is **findable and not writable**: `scorsese icons
/// unlock` answers `lock-open`, and this is the other half of that — the
/// catalogue has one name per icon, and the retired one is refused like any
/// other name nothing ships.
#[test]
fn a_name_upstream_retired_is_refused_like_any_other() {
    let run = checked("icon-retired", "unlock");
    assert!(run.failed, "a retired name draws nothing:\n{}", run.output);
    run.says("there is no icon called `unlock`");
}

/// Nothing to suggest is an honest answer, and the message still has to be a
/// sentence rather than trailing off into an empty list.
#[test]
fn a_name_like_nothing_at_all_still_refuses_cleanly() {
    let run = checked("icon-nothing", "qzxwvjkhgfdsplm");
    assert!(run.failed, "{}", run.output);
    run.says("there is no icon called `qzxwvjkhgfdsplm`");
    run.silent_about("Did you mean");
}
