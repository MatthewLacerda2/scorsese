//! A command whose reader leaves early stops quietly (#996).
//!
//! `scorsese prices | head -1` used to panic — "failed printing to stdout:
//! Broken pipe", exit 101 — and an agent read that as the command failing. The
//! pipe here is closed before the command starts, so its very first line meets
//! a reader that is already gone: no timing to win or lose.

use std::process::{Command, Stdio};

/// Runs `scorsese` with `arguments` into a pipe nobody reads, and returns its
/// exit code and whatever it said on stderr.
fn into_a_closed_pipe(arguments: &[&str]) -> (Option<i32>, String) {
    let (reader, writer) = std::io::pipe().expect("open a pipe");
    drop(reader);
    let output = Command::new(env!("CARGO_BIN_EXE_scorsese"))
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(writer)
        .stderr(Stdio::piped())
        .output()
        .expect("run the scorsese binary");
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn a_reader_that_leaves_ends_the_command_quietly() {
    let (code, stderr) = into_a_closed_pipe(&["prices"]);
    assert_eq!(code, Some(0), "stderr:\n{stderr}");
    assert!(
        stderr.is_empty(),
        "expected nothing on stderr, got:\n{stderr}"
    );
}

#[test]
fn a_command_that_fails_still_fails_into_a_closed_pipe() {
    let (code, stderr) = into_a_closed_pipe(&["guide", "no-such-guide"]);
    assert_eq!(code, Some(1), "stderr:\n{stderr}");
    assert!(stderr.contains("error:"), "stderr:\n{stderr}");
}
