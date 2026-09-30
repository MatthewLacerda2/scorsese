//! The harness the snapshots are drawn through, and the lock that keeps them
//! from drawing at the same time.
//!
//! Apart from the tests themselves because it is machinery rather than a claim
//! about the window: a reader wanting to know what is asserted should meet the
//! four snapshots, not the reason wgpu needs a mutex. The tolerance every
//! snapshot is compared with lives here too, for the same reason — the
//! *decision* behind its numbers is in `main.rs`'s module doc.

use std::ops::{Deref, DerefMut};
use std::sync::{Mutex, MutexGuard};

use egui_kittest::{Harness, OsThreshold, SnapshotOptions};
use scorsese_app::Scorsese;

use crate::fixture;
use crate::watchdog;

/// The window's size in these snapshots.
///
/// The same as the real window opens at, so what a reference shows is what a
/// person sees rather than a squeezed approximation of it.
const WINDOW: [f32; 2] = [1280.0, 800.0];

/// How far one pixel may drift from its reference before it counts as
/// different, on Linux — where the references are blessed and where a merge is
/// gated — and on any platform not named below.
///
/// The unit is dify's own, **not** a fraction: `0.5053·ΔY² + 0.299·ΔI² +
/// 0.1957·ΔQ²`, in YIQ over 0–255 channels. So `0.6` lets a pixel through that
/// is off by at most one level in every channel (worst `0.542`), and stops a
/// two-level shift in grey (`2.02`). It is `egui_kittest`'s own default, and it
/// is what has gated every merge so far — [`tolerance`] says why that needs
/// saying.
const LINUX: f32 = 0.6;

/// The same line on macOS: a pixel may disagree by up to **two levels in each
/// channel**, and no more.
///
/// `2.2` is the worst that model can score (`2.166`, rounded up), and the model
/// is measured rather than assumed. A Mac (Apple silicon, Metal) drawing all
/// twelve panels differs from the Linux references on 2,742 to 30,963 pixels a
/// panel, never by more than two levels in any channel, worst score `1.348`.
/// Three levels in grey scores `4.5`; anything a panel is actually about — a
/// label, a clip, a line moved, a colour changed on purpose — scores in the
/// thousands. The two generate-dialog references, a cent of narration apart,
/// put 2,602 pixels over this line, the worst at `28464`. `main.rs`'s module doc has the argument for this over skipping.
const MACOS: f32 = 2.2;

/// Every snapshot's tolerance, stated in code rather than in a `kittest.toml`.
///
/// There *was* a `kittest.toml`, in `tests/`, saying `threshold = 0.8` and
/// `failed_pixel_count_threshold = 400`. `egui_kittest` looks for that file
/// from the test's working directory **upwards** — which is `app/`, never
/// `app/tests/` — so it was never read, on any machine, and the gate in force
/// was the crate's default the whole time: `0.6`, and not one pixel allowed
/// over it. The file described a tolerance nobody had. Here nothing has to be
/// found for it to apply, and [`LINUX`] is the number that was really gating,
/// so writing it down changes nothing on CI.
///
/// **No pixel over the line, on either platform.** A count of pixels allowed to
/// be wrong would have to be large enough to absorb a rasteriser's text edges
/// and small enough to catch a changed digit, and no number is both; the
/// per-pixel line is where two rasterisers and a real change actually part.
fn tolerance() -> SnapshotOptions {
    SnapshotOptions::new()
        .threshold(OsThreshold::new(LINUX).macos(MACOS))
        .failed_pixel_count_threshold(0)
}

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
///
/// It is not the whole rule, and the half that was missing broke along exactly
/// the line that argument draws. A `static` covers one **process**, which is
/// what `cargo test` gives it; nextest runs a **process per test**, so across
/// those this holds nothing and every snapshot in the binary builds its own
/// device at once. That fails differently — `RequestDeviceError(OutOfMemory)`
/// from `egui_kittest`, no frame drawn and so no `.diff.png` to look at, about
/// one run in two. `app/.config/nextest.toml` is the other half: it pins this
/// binary to one test at a time, and it lives in the runner's own config for
/// the reason above, because the runner reads it however it was invoked. Two
/// runners, two mechanisms, and dropping either brings back its own failure.
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

/// A harness drawing the whole window over `project`, on the machine
/// [`fixture::machine`] describes.
///
/// Every test starts here, which is why the [`watchdog`] is armed, the drawing
/// lock taken and the machine stated here rather than in each test: a snapshot
/// added later cannot forget to do any of the three, and forgetting would
/// restore exactly the failure modes they guard against.
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
            .with_options(tolerance())
            .build_ui_state(
                |ui, window: &mut Scorsese| window.draw(ui),
                Scorsese::opening_with(project, fixture::machine()),
            ),
        _one_at_a_time: one_at_a_time,
    }
}
