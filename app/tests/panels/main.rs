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
//! no other test here can do. [`watchdog`] is why they no longer do.

mod fixture;
mod watchdog;

use std::ops::{Deref, DerefMut};
use std::sync::{Mutex, MutexGuard};

use egui_kittest::Harness;
use scorsese_app::Scorsese;

/// The window's size in these snapshots.
///
/// The same as the real window opens at, so what a reference shows is what a
/// person sees rather than a squeezed approximation of it.
const WINDOW: [f32; 2] = [1280.0, 800.0];

/// Held for the length of a snapshot, so only one is ever being drawn.
///
/// libtest runs tests on a thread each, and **two of these building a wgpu
/// device at the same time deadlock**. Measured rather than guessed: eight runs
/// of the four snapshots one at a time all passed, and three of six runs at the
/// default parallelism wedged instead — every thread spinning, nothing
/// progressing, no error from anything.
///
/// A lock here rather than `--test-threads=1` in the Makefile, because the
/// Makefile is not the only way these get run, and a rule that only holds when
/// invoked the blessed way is a rule that will be broken by someone typing
/// `cargo test`. It costs nothing: the four together take about a second.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// A harness with the drawing lock held.
///
/// The lock has to outlive the harness rather than the call that made it, which
/// is why this exists instead of `window()` simply returning a `Harness`.
/// Everything else about it is a `Harness`, which is what the [`Deref`] pair is
/// for — a test reads the same as it did before this was needed.
struct Drawing {
    harness: Harness<'static, Scorsese>,
    _one_at_a_time: MutexGuard<'static, ()>,
}

impl Deref for Drawing {
    type Target = Harness<'static, Scorsese>;

    fn deref(&self) -> &Self::Target {
        &self.harness
    }
}

impl DerefMut for Drawing {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.harness
    }
}

/// A harness drawing the whole window over `project`.
///
/// Every test starts here, which is why the [`watchdog`] is armed and the
/// drawing lock taken here rather than in each test: a snapshot added later
/// cannot forget to do either, and forgetting would restore exactly the failure
/// mode they guard against.
fn window(project: Option<std::path::PathBuf>) -> Drawing {
    watchdog::arm();
    // A poisoned lock is a snapshot that panicked while holding it — a failure
    // already being reported. Refusing to draw the rest because of it would
    // turn one failing snapshot into four, and hide the three.
    let one_at_a_time = ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    Drawing {
        harness: Harness::builder()
            .with_size(egui::vec2(WINDOW[0], WINDOW[1]))
            .build_ui_state(
                |ui, window: &mut Scorsese| window.draw(ui),
                Scorsese::opening(project),
            ),
        _one_at_a_time: one_at_a_time,
    }
}

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

/// A project that will not load. Every problem at once, in the window rather
/// than in a terminal — the promise validation already makes to the CLI.
#[test]
fn a_project_that_will_not_load() {
    let project = fixture::broken("broken");
    let mut harness = window(Some(project.path().to_path_buf()));
    harness.run();
    harness.snapshot("a_project_that_will_not_load");
}
