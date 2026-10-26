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
//!
//! **Every run is sealed off from the machine running it** ([`sealed`], #1222):
//! no provider key from the tester's shell, no `.env` from the checkout, no
//! settings file from their home, and no way out to the network. A test that
//! spends money is a bug, and the maintainer's own machine holds real keys in
//! all three places.

pub(crate) mod documents;
pub(crate) mod media;
#[path = "../../../../../tools/scratch.rs"]
pub(crate) mod scratch;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use scorsese_core::Project;
use scorsese_providers::credentials::Provider;

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

/// Where outbound HTTP is sent instead of the internet: the discard port on
/// this machine, where nothing listens, so a request is refused at once. The
/// HTTP client reads these variables by default, `ALL_PROXY` first.
const DEAD_PROXY: &str = "http://127.0.0.1:9";

/// The variables a proxy is read from, both cases, since tools disagree.
const PROXY_VARIABLES: [&str; 6] = [
    "ALL_PROXY",
    "all_proxy",
    "HTTPS_PROXY",
    "https_proxy",
    "HTTP_PROXY",
    "http_proxy",
];

/// The `scorsese` binary, sealed off from the machine running the tests.
///
/// - **No provider key** reaches it from the tester's environment: every one
///   [`Provider::ALL`] names is removed, so a new provider is covered the day
///   it is added. The credentials resolver reads the environment *first*, so
///   an exported key would beat any fake settings a test writes.
/// - **No `.env`**: it runs in the system's temporary directory, outside the
///   checkout, so the walk up for one finds nothing.
/// - **No settings file**: the folders one is looked up in point at an empty
///   scratch directory. [`run_in_env`] points them at a test's own instead.
/// - **No network**: requests go to [`DEAD_PROXY`] and are refused, while
///   loopback stays reachable for anything a test serves itself.
pub(crate) fn sealed() -> Command {
    seal(Command::new(env!("CARGO_BIN_EXE_scorsese")))
}

/// Seals `command` as [`sealed`] does, overriding whatever it inherited or
/// was given before — which is how a test stands in for a tester's shell.
pub(crate) fn seal(mut command: Command) -> Command {
    for provider in Provider::ALL {
        command.env_remove(provider.variable());
    }
    let home = scratch::fresh("scorsese-cli-sealed-home", "");
    std::fs::create_dir_all(&home).expect("create the sealed home");
    for name in ["XDG_CONFIG_HOME", "APPDATA", "HOME"] {
        command.env(name, &home);
    }
    for name in PROXY_VARIABLES {
        command.env(name, DEAD_PROXY);
    }
    command
        .env("NO_PROXY", "localhost,127.0.0.1")
        .env("no_proxy", "localhost,127.0.0.1")
        .current_dir(std::env::temp_dir());
    command
}

/// Runs a subcommand against `dir`, which is named with the global
/// `--project` flag rather than by changing directory: tests share a process.
pub(crate) fn run_in(dir: &Path, arguments: &[&str]) -> Run {
    let mut command = sealed();
    command.args(arguments).arg("--project").arg(dir);
    capture(command)
}

/// The same, with extra environment variables set — how a test points a run at
/// a settings file of its own, holding keys that are not keys.
pub(crate) fn run_in_env(dir: &Path, arguments: &[&str], env: &[(&str, &Path)]) -> Run {
    let mut command = sealed();
    command.args(arguments).arg("--project").arg(dir);
    for (name, value) in env {
        command.env(name, value);
    }
    capture(command)
}

/// The same, with ffmpeg and ffprobe pointed at nothing runnable — so a command
/// that reaches for either fails and names it, and one that does not, cannot.
pub(crate) fn run_without_tools(dir: &Path, arguments: &[&str]) -> Run {
    let mut command = sealed();
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
    let mut command = sealed();
    command.arg("new").arg(directory).args(arguments);
    capture(command)
}

pub(crate) fn capture(mut command: Command) -> Run {
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

/// A path in the temporary area that nothing has created yet.
/// Removed when the test ends.
pub(crate) fn scratch(label: &str) -> PathBuf {
    scratch::fresh(&format!("scorsese-cli-{label}"), ".scor")
}

/// Reads the document back through the same load a command takes, so a test
/// asserting on it is asserting on a project that still validates.
pub(crate) fn reload(dir: &Path) -> Project {
    Project::load(dir).expect("the project on disk loads")
}

/// The document as it is on disk, for a test asserting on its text, or that a
/// command left it exactly as it was.
pub(crate) fn document(dir: &Path) -> String {
    std::fs::read_to_string(dir.join(scorsese_core::PROJECT_FILE_NAME)).expect("read project.json")
}

/// True when `dir` holds this file or directory.
pub(crate) fn holds(dir: &Path, entry: impl AsRef<OsStr>) -> bool {
    dir.join(entry.as_ref()).exists()
}
