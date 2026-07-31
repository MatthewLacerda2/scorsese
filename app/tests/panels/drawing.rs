//! The harness the snapshots are drawn through, and the lock that keeps them
//! from drawing at the same time.
//!
//! Apart from the tests themselves because it is machinery rather than a claim
//! about the window: a reader wanting to know what is asserted should meet the
//! four snapshots, not the reason wgpu needs a mutex.

use std::ops::{Deref, DerefMut};
use std::sync::{Mutex, MutexGuard};

use egui_kittest::Harness;
use scorsese_app::Scorsese;

use crate::watchdog;

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
pub(crate) struct Drawing {
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
pub(crate) fn window(project: Option<std::path::PathBuf>) -> Drawing {
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
