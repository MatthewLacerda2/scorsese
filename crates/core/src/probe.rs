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
    /// Inspects a file on disk. Everything a probe cannot determine comes
    /// back absent rather than guessed — an unset field means "not known",
    /// which is not the same as zero.
    fn probe(&self, file: &Path) -> Result<MediaMetadata, ProbeError>;
}

/// A probe that failed. The message is whatever the prober wants to say;
/// core deliberately knows nothing about ffprobe's failure modes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("could not probe {}: {message}", path.display())]
pub struct ProbeError {
    /// The file that was being probed. A real filesystem path, not a
    /// [`crate::ProjectPath`]: the source of an import is outside the project.
    pub path: PathBuf,
    /// The prober's own account of what went wrong, passed through verbatim.
    pub message: String,
}

impl ProbeError {
    /// Builds a failure for a prober that has something to say about a file.
    pub fn new(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}
