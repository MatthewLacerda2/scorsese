//! What ffprobe says about a file a render wrote.
//!
//! This is how a test asserts that a setting reached the encoder rather than
//! that the CLI printed it: the width, the rate and the sample rate come off
//! the delivered file, which is the only place they mean anything.

use std::path::{Path, PathBuf};

use scorsese_render::Tools;

/// What ffprobe reported about one stream, as `key=value` lines.
pub(crate) struct Probed {
    file: PathBuf,
    entries: String,
}

impl Probed {
    /// The value recorded for `key`, panicking with the whole report when the
    /// stream does not carry it — including when there is no such stream.
    #[track_caller]
    pub(crate) fn get(&self, key: &str) -> &str {
        self.entries
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .map(str::trim)
            .unwrap_or_else(|| {
                panic!(
                    "no `{key}` for {}, only:\n{}",
                    self.file.display(),
                    self.entries
                )
            })
    }

    #[track_caller]
    pub(crate) fn number(&self, key: &str) -> u64 {
        let value = self.get(key);
        value
            .parse()
            .unwrap_or_else(|_| panic!("`{key}` is `{value}`, which is not a number"))
    }

    /// Whether the file carries a stream of this kind at all.
    pub(crate) fn exists(&self) -> bool {
        !self.entries.trim().is_empty()
    }
}

/// Asks ffprobe for `entries` of the first `stream` (`v:0` or `a:0`) of a file,
/// counting frames rather than trusting the container's own claim about them.
pub(crate) fn probe(tools: &Tools, file: &Path, stream: &str, entries: &str) -> Probed {
    let output = tools
        .ffprobe()
        .args(["-v", "error", "-select_streams", stream, "-count_frames"])
        .args(["-show_entries", entries])
        .args(["-of", "default=noprint_wrappers=1"])
        .arg(file)
        .output()
        .expect("run ffprobe");
    assert!(
        output.status.success(),
        "ffprobe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Probed {
        file: file.to_path_buf(),
        entries: String::from_utf8_lossy(&output.stdout).into_owned(),
    }
}
