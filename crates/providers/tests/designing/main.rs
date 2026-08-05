//! Designing a voice, driven end to end against a scripted studio.
//!
//! **Nothing here reaches a network, needs a key, or spends a cent**, and that
//! is the point of [`Studio`](scorsese_providers::voices::design::Studio) being
//! a trait. It is also what makes the interesting states reachable at all: an
//! account whose plan does not include Voice Design, a ceiling that refuses a
//! run, and a candidate that no design offered would each otherwise need
//! somebody's billing details.
//!
//! The three things asserted here are the three the issue was written about:
//! the money is right and is counted once, the samples are never paid for
//! twice, and what made a voice is written down where the voice itself cannot
//! travel.

#[path = "../common/mod.rs"]
mod common;

mod fake;
mod keeping;
mod money;

use scorsese_providers::voices::design::Brief;

use common::project;

/// A passage long enough for the vendor's floor, and a description likewise.
///
/// Written as helpers rather than as constants at each call site so a test says
/// what it is about instead of what it had to satisfy — the lengths are the
/// vendor's business and are asserted on their own in `money`.
fn brief(prompt: &str, seed: Option<u32>) -> Brief {
    Brief::new(
        &format!("{prompt}, unhurried and warm"),
        &passage(),
        seed,
        None,
    )
    .expect("a brief inside the vendor's lengths")
}

/// A hundred and something characters for the candidates to read.
fn passage() -> String {
    String::from(
        "The tide came in slowly that evening, and nobody on the beach thought to \
         move until the light had gone entirely.",
    )
}

/// A project directory to design into, and nothing else in it.
fn root(label: &str) -> std::path::PathBuf {
    project(label).0
}
