//! Asking ffprobe what is inside a media file.

use std::path::Path;

use scorsese_core::asset::MediaMetadata;
use scorsese_core::probe::{ProbeError, ProbeMedia};

use crate::tools::Tools;

use super::report::Report;

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

impl ProbeMedia for Ffprobe {
    fn probe(&self, file: &Path) -> Result<MediaMetadata, ProbeError> {
        let output = self
            .tools
            .ffprobe()
            .args([
                "-v",
                "error",
                "-print_format",
                "json",
                "-show_format",
                "-show_streams",
            ])
            .arg(file)
            .output()
            .map_err(|error| ProbeError::new(file, error.to_string()))?;

        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr);
            let message = message.trim();
            let message = if message.is_empty() {
                "ffprobe reported no detail"
            } else {
                message
            };
            return Err(ProbeError::new(file, message));
        }

        let report: Report = serde_json::from_slice(&output.stdout).map_err(|error| {
            ProbeError::new(file, format!("unreadable ffprobe output: {error}"))
        })?;
        Ok(report.into_metadata())
    }
}
