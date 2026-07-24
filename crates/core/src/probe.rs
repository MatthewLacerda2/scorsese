//! The seam between the project model and whatever inspects media files.

use std::path::{Path, PathBuf};

use crate::asset::MediaMetadata;

/// Reads technical metadata out of a media file.
///
/// Core declares this and never implements it: probing means running ffprobe,
/// and this crate must not spawn a process. `scorsese-render` supplies the
/// real implementation, and tests supply a stub — which is why importing can
/// be tested without ffmpeg installed at all.
pub trait ProbeMedia {
    fn probe(&self, file: &Path) -> Result<MediaMetadata, ProbeError>;
}

/// A probe that failed. The message is whatever the prober wants to say;
/// core deliberately knows nothing about ffprobe's failure modes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("could not probe {}: {message}", path.display())]
pub struct ProbeError {
    pub path: PathBuf,
    pub message: String,
}

impl ProbeError {
    pub fn new(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}
