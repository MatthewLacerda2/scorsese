//! A real project directory in a temporary directory, and the real `scorsese`
//! run over it.
//!
//! Every test built on this goes through the compiled binary rather than
//! calling the function behind it: the exit code is what an unattended caller
//! branches on, and only a process has one. What a test then asserts is the
//! **effect** — what is on disk afterwards, what the document says, what the
//! encoded file contains — rather than the wording of the report, which is the
//! one part of a CLI that is allowed to change.
//!
//! The exception is a report that *is* the product: `assets` exists to say what
//! the pool holds, and a render's notes are how an unattended run learns it
//! went wrong. Those are asserted on by substance — the id, the state, the clip
//! named — never by whole lines.
#![allow(dead_code)]

pub(crate) mod documents;
pub(crate) mod media;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use scorsese_core::Project;

/// Nothing runnable, on any platform this builds for. A command pointed at it
/// that reaches for ffmpeg says so, which is what lets a test prove one didn't.
pub(crate) const NO_FFMPEG: &str = "scorsese-tests-no-such-ffmpeg";

/// What one run of the binary said, and whether it failed.
pub(crate) struct Run {
    /// The exit status, as the shell would read it.
    pub(crate) failed: bool,
    /// Both streams, joined: problems go to stderr and everything else to
    /// stdout, and a test cares about the whole report.
    pub(crate) output: String,
}

impl Run {
    #[track_caller]
    pub(crate) fn says(&self, needle: &str) {
        assert!(
            self.output.contains(needle),
            "expected `{needle}` in:\n{}",
            self.output
        );
    }

    #[track_caller]
    pub(crate) fn silent_about(&self, needle: &str) {
        assert!(
            !self.output.contains(needle),
            "did not expect `{needle}` in:\n{}",
            self.output
        );
    }

    /// Asserts the run succeeded, printing what it said when it did not — the
    /// assertion every test that goes on to inspect the effect starts with.
    #[track_caller]
    pub(crate) fn ok(self) -> Self {
        assert!(!self.failed, "the run failed:\n{}", self.output);
        self
    }
}

/// Runs a subcommand against `dir`, which is named with the global
/// `--project` flag rather than by changing directory: tests share a process.
pub(crate) fn run_in(dir: &Path, arguments: &[&str]) -> Run {
    let mut command = Command::new(env!("CARGO_BIN_EXE_scorsese"));
    command.args(arguments).arg("--project").arg(dir);
    capture(command)
}

/// The same, with ffmpeg and ffprobe pointed at nothing runnable — so a command
/// that reaches for either fails and names it, and one that does not, cannot.
pub(crate) fn run_without_tools(dir: &Path, arguments: &[&str]) -> Run {
    let mut command = Command::new(env!("CARGO_BIN_EXE_scorsese"));
    command
        .args(arguments)
        .arg("--project")
        .arg(dir)
        .env(scorsese_render::tools::FFMPEG_ENV, NO_FFMPEG)
        .env(scorsese_render::tools::FFPROBE_ENV, NO_FFMPEG);
    capture(command)
}

/// Runs `scorsese new <directory>`, where the directory is a positional
/// argument because the project does not exist yet to be pointed at.
pub(crate) fn new_at(directory: &Path, arguments: &[&str]) -> Run {
    let mut command = Command::new(env!("CARGO_BIN_EXE_scorsese"));
    command.arg("new").arg(directory).args(arguments);
    capture(command)
}

fn capture(mut command: Command) -> Run {
    let output = command.output().expect("run the scorsese binary");
    Run {
        failed: !output.status.success(),
        output: format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    }
}

/// A project directory laid out by `scorsese new` itself — the same starting
/// point a person or an agent has.
pub(crate) fn new_project(label: &str) -> PathBuf {
    let dir = scratch(label);
    new_at(&dir, &[]).ok();
    dir
}

/// An empty temporary directory, unique per call.
pub(crate) fn temp_dir(label: &str) -> PathBuf {
    let dir = scratch(label);
    std::fs::create_dir_all(&dir).expect("create the temporary directory");
    dir
}

/// A path in the temporary area that nothing has created yet.
pub(crate) fn scratch(label: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "scorsese-cli-{label}-{}-{unique}.scor",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// Reads the document back through the same load a command takes, so a test
/// asserting on it is asserting on a project that still validates.
pub(crate) fn reload(dir: &Path) -> Project {
    Project::load(dir).expect("the project on disk loads")
}

/// True when `dir` holds this file or directory.
pub(crate) fn holds(dir: &Path, entry: impl AsRef<OsStr>) -> bool {
    dir.join(entry.as_ref()).exists()
}
