//! `scorsese import a b c` — media arrives in sets.
//!
//! What matters beyond "all three landed" is the partial failure: one bad path
//! among good ones must not cost the good ones, and must not be reported as
//! success either.

use std::path::Path;

use crate::common::{Run, reload, run_in};

use super::{CLIP, Outside};

/// Two more clips, each distinguishable from [`CLIP`] by its bytes.
const BLUE: &str = "color=c=blue:s=64x64:d=2:r=30";
const GREEN: &str = "color=c=green:s=64x64:d=2:r=30";

/// One `import` naming every path at once.
fn import_all(outside: &Outside, files: &[&Path]) -> Run {
    let mut all = vec!["import"];
    all.extend(files.iter().map(|f| f.to_str().expect("a utf-8 path")));
    run_in(&outside.project, &all)
}

#[test]
fn several_files_come_in_from_one_call_and_are_tallied() {
    let outside = Outside::new("several");
    let wide = outside.media("wide.mp4", CLIP);
    let close = outside.media("close.mp4", BLUE);
    let still = outside.still("card.png");

    let run = import_all(&outside, &[&wide, &close, &still]).ok();
    run.says("wide — Video");
    run.says("close — Video");
    run.says("card — Image");
    // The tally exists because the per-asset lines above are too long to count
    // by eye once a set is more than a handful.
    run.says("3 imported, 0 already in the pool, 0 skipped");

    assert_eq!(outside.pool(), ["card.png", "close.mp4", "wide.mp4"]);
    assert_eq!(reload(&outside.project).assets.len(), 3);
    outside.clean();
}

#[test]
fn one_path_is_still_reported_without_a_tally() {
    // The tally is noise when there is one line above it to count.
    let outside = Outside::new("single");
    let wide = outside.media("wide.mp4", CLIP);

    let run = import_all(&outside, &[&wide]).ok();
    run.says("wide — Video");
    run.silent_about("already in the pool,");
    outside.clean();
}

#[test]
fn a_path_that_fails_does_not_cost_the_ones_beside_it() {
    // The whole point of the set: a mistyped name in the middle of five is a
    // typo, not a reason to import nothing.
    let outside = Outside::new("partial");
    let wide = outside.media("wide.mp4", CLIP);
    let close = outside.media("close.mp4", GREEN);
    let missing = outside.elsewhere.join("not-here.mp4");

    let run = import_all(&outside, &[&wide, &missing, &close]);
    assert!(
        run.failed,
        "a partial import is not a whole one:\n{}",
        run.output
    );
    run.says("not-here.mp4 — failed");
    run.says("1 of 3 path(s) failed to import");

    // Both good ones landed, and landed on disk — the save happens once, after
    // the loop, so the failure in the middle must not have discarded the first.
    assert_eq!(outside.pool(), ["close.mp4", "wide.mp4"]);
    assert_eq!(reload(&outside.project).assets.len(), 2);
    outside.clean();
}

#[test]
fn naming_no_path_at_all_is_refused_by_the_parser() {
    // `required = true` on a variadic argument: without it, a bare `import`
    // would quietly succeed at importing nothing.
    let outside = Outside::new("nopaths");
    let run = run_in(&outside.project, &["import"]);
    assert!(
        run.failed,
        "a bare import has nothing to do:\n{}",
        run.output
    );
    outside.clean();
}
