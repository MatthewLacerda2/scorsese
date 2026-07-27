//! What `check` does to the exit code, run as a real process.
//!
//! The compatibility constraint is the point of most of these: nothing that
//! renders today may start failing, so every shape that renders is asserted
//! to still pass, not merely to print something reasonable.

use crate::common::{check, clip, media_file, project, sketch, unprobed_video, video};

#[test]
fn a_healthy_project_passes_with_nothing_to_say() {
    let dir = project("healthy", &[video("boat")], &[clip("c1", "boat", 0)]);
    media_file(&dir, "boat", b"");

    let run = check(&dir, false);
    assert!(!run.failed, "{}", run.output);
    run.says("no problems, nothing to warn about");
}

#[test]
fn a_file_a_clip_needs_going_missing_fails() {
    // The gap this command exists to close: the document is perfectly
    // coherent, and the render would die partway through anyway.
    let dir = project("missing", &[video("boat")], &[clip("c1", "boat", 0)]);

    let run = check(&dir, false);
    assert!(
        run.failed,
        "a clip with no file to decode must fail:\n{}",
        run.output
    );
    run.says("boat");
    run.says("its file is not there");
}

#[test]
fn a_sketch_project_before_go_passes_silently() {
    // A prompt clip with no media yet is the normal state of a project before
    // GO, and must never read as broken.
    let dir = project("sketch", &[sketch("boat")], &[clip("c1", "boat", 0)]);

    let run = check(&dir, false);
    assert!(!run.failed, "{}", run.output);
    run.says("no problems, nothing to warn about");
    run.silent_about("warning");
}

#[test]
fn a_missing_file_nothing_references_warns_but_passes() {
    // `gc` is the answer to this, not a failed check: no clip can be hurt by
    // a file no clip reads.
    let dir = project(
        "orphan",
        &[video("boat"), video("orphan")],
        &[clip("c1", "boat", 0)],
    );
    media_file(&dir, "boat", b"");

    let run = check(&dir, false);
    assert!(!run.failed, "{}", run.output);
    run.says("warning: asset `orphan`");
    run.says("no problems, 1 warning(s)");
}

#[test]
fn an_unprobed_file_warns_but_passes() {
    let dir = project(
        "unprobed",
        &[unprobed_video("boat")],
        &[clip("c1", "boat", 0)],
    );
    media_file(&dir, "boat", b"");

    let run = check(&dir, false);
    assert!(!run.failed, "{}", run.output);
    run.says("warning: asset `boat`");
    run.says("no problems, 1 warning(s)");
}

#[test]
fn a_document_problem_still_fails_the_way_it_always_did() {
    let dir = project("dangling", &[video("boat")], &[clip("c1", "ghost", 0)]);
    media_file(&dir, "boat", b"");

    let run = check(&dir, false);
    assert!(run.failed, "{}", run.output);
    run.says("1 problem in this project:");
    run.says("is not in the assets table");
}
