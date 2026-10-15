//! A test run of `scorsese` cannot spend money, even on a machine that holds
//! real keys (#1222).
//!
//! The harness seals every run off from the machine ([`common::sealed`]); these
//! tests hold it to that. The tester's shell is stood in for by setting a
//! variable on the command *before* it is sealed, which is what inheriting it
//! amounts to — setting it on this process instead would leak into every
//! other test sharing it.

#[path = "common/mod.rs"]
mod common;

use std::process::Command;

use common::{capture, document, documents, seal, sealed};
use scorsese_providers::credentials::Provider;

#[test]
fn a_key_exported_in_the_testers_shell_never_reaches_a_run() {
    let mut shell = Command::new(env!("CARGO_BIN_EXE_scorsese"));
    for provider in Provider::ALL {
        shell.env(provider.variable(), "a-real-key");
    }
    let mut command = seal(shell);
    command.arg("settings");

    let run = capture(command).ok();

    for provider in Provider::ALL {
        run.silent_about(&format!("found, from {}", provider.variable()));
    }
    // Nor did it find a `.env` or the tester's settings file to fall back on.
    run.says(".env:          none found");
    run.silent_about("found, from");
}

/// The second wall, for the key that gets past the first: a run that does
/// reach for a provider finds the network refused, and submits nothing.
#[test]
fn a_run_that_reaches_for_a_provider_is_refused_the_network() {
    let dir = documents::project(
        "sealed-network",
        &[documents::assets::sketch("shot")],
        &[documents::clip("c1", "shot", 0)],
    );
    let mut command = sealed();
    command
        .args(["generate", "--yes", "--project"])
        .arg(&dir)
        .env(Provider::Gemini.variable(), "not-a-key");

    let run = capture(command);

    assert!(run.failed, "the run reached a provider:\n{}", run.output);
    // The transport failed on this machine — never an answer from the vendor,
    // which would mean the request left it.
    run.says("could not reach");
    let document = document(&dir);
    assert!(
        !document.contains("operation"),
        "the shot was submitted:\n{document}"
    );
}
