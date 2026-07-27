//! When `scorsese render` refuses a format — which matters as much as whether.
//!
//! A refusal that arrives after the encode has already run is not a fix for
//! the problem it is meant to fix: the cost of a wrong-shaped file is the
//! minutes spent producing it, and for an unattended agent, the plausible
//! result it then acts on.
//!
//! So these runs point ffmpeg and ffprobe at a binary that does not exist. A
//! refusal that reaches the tools says so — `Tools::discover` complains about
//! ffmpeg by name — and a refusal that beats them cannot. The control test
//! below is what gives the silence its meaning.

mod refusals;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

/// Nothing runnable, on any platform this builds for.
pub(crate) const NO_FFMPEG: &str = "scorsese-tests-no-such-ffmpeg";

/// What `Tools::discover` says when it cannot find them, in the words the
/// error itself uses.
pub(crate) const LOOKED_FOR_FFMPEG: &str = "install ffmpeg";

pub(crate) struct Run {
    pub(crate) failed: bool,
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
}

/// A real, empty project — enough for a render to get as far as looking for
/// ffmpeg, which is exactly as far as these tests care about.
pub(crate) fn project(label: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "scorsese-cli-{label}-{}-{unique}.scor",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let made = Command::new(env!("CARGO_BIN_EXE_scorsese"))
        .arg("new")
        .arg(&dir)
        .output()
        .expect("run scorsese new");
    assert!(
        made.status.success(),
        "scorsese new failed: {}",
        String::from_utf8_lossy(&made.stderr)
    );
    dir
}

pub(crate) fn render(dir: &Path, arguments: &[&str]) -> Run {
    let output = Command::new(env!("CARGO_BIN_EXE_scorsese"))
        .arg("render")
        .arg("--project")
        .arg(dir)
        .args(arguments)
        .env("SCORSESE_FFMPEG", NO_FFMPEG)
        .env("SCORSESE_FFPROBE", NO_FFMPEG)
        .output()
        .expect("run scorsese render");
    Run {
        failed: !output.status.success(),
        output: format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    }
}
