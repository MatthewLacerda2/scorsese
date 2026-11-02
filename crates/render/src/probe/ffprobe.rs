//! Asking ffprobe what is inside a media file.

use std::path::Path;

use scorsese_core::MediaMetadata;
use scorsese_core::probe::{ProbeError, ProbeMedia};

use crate::tools::{Tools, input};

use super::report::{Lengths, Report};

/// Probes media by running `ffprobe`.
///
/// This is the implementation of core's [`ProbeMedia`] seam: core declares
/// what a probe is, and this crate is the only thing that knows a probe means
/// a subprocess.
#[derive(Debug, Clone)]
pub struct Ffprobe {
    tools: Tools,
}

impl Ffprobe {
    /// Probes using tools that have already been located — the constructor for
    /// a caller that holds a [`Tools`] and should not go looking twice.
    pub fn new(tools: Tools) -> Self {
        Self { tools }
    }

    /// Finds ffprobe the usual way.
    pub fn discover() -> Result<Self, crate::tools::ToolsError> {
        Ok(Self::new(Tools::discover()?))
    }
}

impl Ffprobe {
    /// How long `file` says it plays, container and streams apart, for a
    /// caller that must not trust one file's word for its own length (the
    /// web app's uploads, #1634).
    pub fn lengths(&self, file: &Path) -> Result<Lengths, ProbeError> {
        Ok(self.report(file)?.lengths())
    }

    /// ffprobe's report on `file`.
    fn report(&self, file: &Path) -> Result<Report, ProbeError> {
        let mut command = self.tools.ffprobe();
        command.args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ]);
        let output = input::media(&mut command, file)
            .output()
            .map_err(|error| ProbeError::new(file, error.to_string()))?;

        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr);
            if refused_listing(&message) {
                return Err(ProbeError::listing(file));
            }
            let message = message.trim();
            let message = if message.is_empty() {
                "ffprobe reported no detail"
            } else {
                message
            };
            return Err(ProbeError::new(file, message));
        }

        serde_json::from_slice(&output.stdout)
            .map_err(|error| ProbeError::new(file, format!("unreadable ffprobe output: {error}")))
    }
}

impl ProbeMedia for Ffprobe {
    fn probe(&self, file: &Path) -> Result<MediaMetadata, ProbeError> {
        Ok(self.report(file)?.into_metadata())
    }
}

/// Whether ffprobe's complaint is the whitelist refusing a demuxer that opens
/// other inputs: it names the demuxer it picked as `[concat @ 0x…]` before
/// `Format not on whitelist`.
fn refused_listing(stderr: &str) -> bool {
    stderr.lines().any(|line| {
        line.contains("not on whitelist")
            && line
                .strip_prefix('[')
                .and_then(|rest| rest.split_once(" @"))
                .is_some_and(|(format, _)| input::LISTING_FORMATS.contains(&format))
    })
}

#[cfg(test)]
mod tests {
    use super::refused_listing;

    #[test]
    fn a_listing_is_told_from_any_other_refused_format() {
        assert!(refused_listing(
            "[concat @ 0x55e] Format not on whitelist 'mov,matroska'\nlist.mp4: Invalid argument\n"
        ));
        assert!(refused_listing("[hls @ 0x1] Format not on whitelist 'mov'"));
        assert!(!refused_listing(
            "[nut @ 0x1] Format not on whitelist 'mov'"
        ));
        assert!(!refused_listing("[concat @ 0x1] Unsafe file name 'x'"));
        assert!(!refused_listing("moov atom not found"));
    }
}
