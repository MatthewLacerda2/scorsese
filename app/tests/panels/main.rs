//! The panels, rendered offscreen and held to reference images.
//!
//! Six panels were built before this existed and **not one frame of any of
//! them had ever been looked at**, because the window could only be drawn by
//! an event loop and this machine has no display. Every test until now
//! deliberately covered the logic *behind* the drawing, because the drawing
//! was unreachable.
//!
//! `egui_kittest` reaches it: it drives the window's `draw` through wgpu with
//! no window and no display, and hands back an image. So the same argument
//! `docs/golden-renders.md` makes about renders applies here — a GPU
//! rasterising text is no more deterministic than an encoder, so these compare
//! **with tolerance** and never byte-for-byte.
//!
//! And the rule that matters carries over unchanged: **re-blessing a reference
//! to make a test pass is never legitimate.** A snapshot changes when the
//! interface was meant to change, and the new picture is looked at before it is
//! committed.
//!
//! Drawing through a GPU also means these can fail by never finishing, which
//! no other test here can do. [`watchdog`] is why they no longer do, and
//! [`drawing`] is the harness they all start from.

mod drawing;
mod fixture;
mod watchdog;

use drawing::window;

/// The window before anything is open — the first thing anyone sees, and the
/// one state that has to invite rather than look broken.
#[test]
fn nothing_open() {
    let mut harness = window(None);
    harness.run();
    harness.snapshot("nothing_open");
}

/// A whole edit: a title over a colour, music under narration, a duck already
/// written, and two clips nobody has generated.
#[test]
fn a_whole_edit() {
    let project = fixture::project("whole");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.run();
    harness.snapshot("a_whole_edit");
}

/// A clip selected: the inspector stops saying "select a clip" and starts
/// saying what one is.
#[test]
fn a_clip_selected() {
    let project = fixture::project("selected");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.state_mut().select("c-title");
    harness.run();
    harness.snapshot("a_clip_selected");
}

/// Several clips selected, on two tracks: every one of them outlined in the
/// timeline, and an inspector that says how many and how far they reach rather
/// than pretending a field could be about all three.
#[test]
fn several_clips_selected() {
    let project = fixture::project("several");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.state_mut().select("c-shot");
    harness.state_mut().also_select("c-title");
    harness.state_mut().also_select("c-vo");
    harness.run();
    harness.snapshot("several_clips_selected");
}

/// A project that will not load. Every problem at once, in the window rather
/// than in a terminal — the promise validation already makes to the CLI.
#[test]
fn a_project_that_will_not_load() {
    let project = fixture::broken("broken");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.run();
    harness.snapshot("a_project_that_will_not_load");
}
