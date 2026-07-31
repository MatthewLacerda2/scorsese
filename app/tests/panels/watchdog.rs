//! A ceiling on how long the snapshots may take before the run is a failure.
//!
//! These tests draw through wgpu, and a machine that cannot get a working
//! adapter does not find that out — it **waits**. Observed on the development
//! machine: `cargo test --manifest-path app/Cargo.toml` sat at 0.5% CPU with
//! two of the four snapshots "running for over 60 seconds", and was still
//! sitting there twenty-five minutes later. Nothing was printed, nothing
//! failed, and nothing was going to.
//!
//! That is the worst outcome available. A red gate is information; a gate that
//! never returns is an unattended agent burning the night on a futex with no
//! signal at either end. `make app` already treats a missing build dependency
//! as something to say out loud rather than fail obscurely — the ALSA check
//! names the package for three distributions — and a silent forever-hang is
//! worse than the obscure panic that check exists to prevent.
//!
//! **It also leaks.** Killing `make` kills `make`; the test binary it spawned
//! survives, and a snapshot process that is spinning goes on spinning. Three
//! were found on the development machine at 100% CPU each, orphaned three days
//! earlier by exactly this hang — and *they* were why the next run hung too,
//! so one hang quietly guaranteed the following ones. A budget the binary
//! enforces on itself is what breaks that cycle: it exits, so there is nothing
//! left to orphan.
//!
//! Deliberately **not** a check that runs first and passes or fails. Whether
//! the adapter is absent, or present and wedged, or contended by a process
//! left over from the last run, the symptom is identical and the answer is the
//! same — stop, and say what to look at. A precondition check would have to
//! predict the cause, and this one has already been the wrong cause once: the
//! development machine has Vulkan ICDs installed and hung anyway.

use std::io::{self, Write};
use std::sync::Once;
use std::time::Duration;
use std::{process, thread};

/// How long the whole snapshot suite gets.
///
/// It runs in about a second on a machine that can draw, and CI's whole
/// `desktop app` job — compile included — is well under two minutes. So this
/// is not a performance budget and must never read as one: it is the line past
/// which "slow" is no longer a plausible explanation, set high enough that a
/// loaded runner on a software rasteriser can never reach it by being slow.
const BUDGET: Duration = Duration::from_secs(300);

/// Shortens the budget, so the guard itself can be watched firing.
///
/// A guard nobody has seen fire is the same thing this repo refuses everywhere
/// else: something that reads as coverage while proving nothing. It exists for
/// [`the_watchdog_stops_a_run_that_will_not_finish`], which re-runs this binary
/// with the variable set and a deliberately wedged test inside it.
pub(crate) const BUDGET_VAR: &str = "PANELS_WATCHDOG_BUDGET_SECONDS";

/// Starts the countdown, once per test binary.
///
/// Every test calls this, and the first call arms it. The thread dies with the
/// process, so a suite that finishes never notices it was there.
pub(crate) fn arm() {
    static ARMED: Once = Once::new();
    ARMED.call_once(|| {
        let budget = std::env::var(BUDGET_VAR)
            .ok()
            .and_then(|seconds| seconds.parse().ok())
            .map_or(BUDGET, Duration::from_secs);
        thread::spawn(move || {
            thread::sleep(budget);
            report(budget);
            // `abort`, not a panic and not `exit`. A panic here would unwind
            // *this* thread and leave the wedged one exactly as it was. `exit`
            // runs the process's cleanup, and the thread that is stuck is stuck
            // inside a graphics driver holding whatever it holds — so the one
            // path out must not be a path that can block on it. Aborting is
            // ugly and it is the only thing guaranteed to happen.
            process::abort();
        });
    });
}

/// What to look at, in the order worth looking.
///
/// Written to the **real** standard error rather than with `eprintln!`, and
/// that is not a style choice: libtest captures the output of a test, and the
/// capture is inherited by threads the test spawns. An `eprintln!` from here
/// therefore lands in a buffer libtest prints when the test *finishes*, which
/// is precisely what is never going to happen. The whole message was swallowed
/// until this went through [`io::stderr`] instead, and a refusal nobody can
/// read is not a refusal.
fn report(budget: Duration) {
    let seconds = budget.as_secs();
    let said = format!(
        "\npanels: the snapshots have been drawing for {seconds}s and are not going to finish.
        They render through wgpu, so something below is true:

        - A snapshot process from an earlier run is still alive and holding the
          driver. Check for one: pgrep -fa panels-  (it will be at ~100% CPU)
        - There is no usable Vulkan adapter. A machine with no GPU needs a
          software rasteriser, which is what CI installs:
            Debian/Ubuntu: sudo apt-get install mesa-vulkan-drivers libvulkan1
            Arch:          sudo pacman -S vulkan-swrast
            Fedora:        sudo dnf install mesa-vulkan-drivers

        This is a refusal, not a skip: a gate that passes by drawing nothing
        would still read as coverage. Fix the adapter and run it again.\n"
    );
    // Nothing useful to do if even this fails, and the exit that follows is the
    // part that must happen regardless.
    let _ = io::stderr().write_all(said.as_bytes());
}

/// The wedged test the self-test below runs, and the only thing `--ignored`
/// covers in this binary.
///
/// It stands in for a snapshot that will not finish: the watchdog cannot tell
/// the difference, which is the point — it does not diagnose, it stops.
#[test]
#[ignore = "hangs on purpose; run by the_watchdog_stops_a_run_that_will_not_finish"]
fn wedged_on_purpose() {
    arm();
    thread::sleep(Duration::from_secs(120));
    panic!("the watchdog should have ended this process long before now");
}

/// The guard, watched firing.
///
/// Runs this same binary again with a one-second budget and the wedged test
/// above, and holds it to the two things that were wrong before: it **ends**,
/// and it **says what to look at**. A budget of 1 against a sleep of 120 means
/// a pass here cannot be the test merely finishing on its own.
#[test]
fn the_watchdog_stops_a_run_that_will_not_finish() {
    let binary = std::env::current_exe().expect("the test binary knows its own path");
    let run = process::Command::new(binary)
        // Filtered by substring rather than `--exact`, because the exact name
        // carries this module's path and a rename would then quietly select no
        // test at all — which passes, and is the shape of failure this whole
        // file is about.
        .args(["--ignored", "wedged_on_purpose"])
        .env(BUDGET_VAR, "1")
        .output()
        .expect("the test binary re-runs");

    assert!(!run.status.success(), "a wedged run must fail, not pass");
    let said = String::from_utf8_lossy(&run.stderr);
    for expected in ["are not going to finish", "pgrep -fa panels-", "vulkan-swrast"] {
        assert!(
            said.contains(expected),
            "the refusal should mention `{expected}`, and said:\n{said}"
        );
    }
}
