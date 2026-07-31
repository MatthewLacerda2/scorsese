//! The guard, watched firing.
//!
//! A guard nobody has seen fire is the same thing this repo refuses everywhere
//! else: something that reads as coverage while proving nothing. So the budget
//! is exercised the only way it can be — by wedging a run on purpose and
//! watching what happens to it.

use std::process::Command;
use std::thread;
use std::time::Duration;

use super::{BUDGET_VAR, arm};

/// The wedged run [`the_watchdog_stops_a_run_that_will_not_finish`] starts, and
/// the only thing `--ignored` covers in this binary.
///
/// It stands in for a snapshot that will not finish. The watchdog cannot tell
/// the difference between this and a real one, which is the point — it does not
/// diagnose, it stops.
#[test]
#[ignore = "hangs on purpose; run by the_watchdog_stops_a_run_that_will_not_finish"]
fn wedged_on_purpose() {
    arm();
    thread::sleep(Duration::from_secs(120));
    panic!("the watchdog should have ended this process long before now");
}

/// Runs this same binary again with a one-second budget and the wedged test
/// above, and holds it to the two things that were wrong before: it **ends**,
/// and it **says what to look at**.
///
/// A budget of one second against a sleep of two minutes is what makes a pass
/// here mean something: it cannot be the run merely finishing on its own.
#[test]
fn the_watchdog_stops_a_run_that_will_not_finish() {
    let binary = std::env::current_exe().expect("the test binary knows its own path");
    let run = Command::new(binary)
        // Filtered by substring rather than `--exact`, because the exact name
        // carries this module's path and a rename would then quietly select no
        // test at all — which passes, and is the shape of failure this whole
        // directory is about.
        .args(["--ignored", "wedged_on_purpose"])
        .env(BUDGET_VAR, "1")
        .output()
        .expect("the test binary re-runs");

    assert!(!run.status.success(), "a wedged run must fail, not pass");
    let said = String::from_utf8_lossy(&run.stderr);
    for expected in [
        "are not going to finish",
        "pgrep -fa panels-",
        "vulkan-swrast",
    ] {
        assert!(
            said.contains(expected),
            "the refusal should mention `{expected}`, and said:\n{said}"
        );
    }
}
