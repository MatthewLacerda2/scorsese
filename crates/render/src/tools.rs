//! Locating ffmpeg and ffprobe, and building commands for them.
//!
//! This is the one place in the workspace that names those binaries. In
//! dev and CI they are on `PATH`; in a shipped build they are Tauri
//! sidecars next to the executable. Everything above this file asks
//! [`Tools`] for a command and never constructs one itself.

use std::path::PathBuf;
use std::process::Command;

/// Overrides for the binaries, so a bundled build can point at its sidecars
/// without changing any calling code.
pub const FFMPEG_ENV: &str = "SCORSESE_FFMPEG";
pub const FFPROBE_ENV: &str = "SCORSESE_FFPROBE";

/// The external tools this crate drives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tools {
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
}

impl Tools {
    /// Finds the tools on `PATH`, honouring the environment overrides, and
    /// checks they actually run. Failing here — with a message that says what
    /// to install — beats failing later inside a half-built render.
    pub fn discover() -> Result<Self, ToolsError> {
        let tools = Self {
            ffmpeg: binary_from_env(FFMPEG_ENV, "ffmpeg"),
            ffprobe: binary_from_env(FFPROBE_ENV, "ffprobe"),
        };
        tools.check(&tools.ffmpeg, FFMPEG_ENV)?;
        tools.check(&tools.ffprobe, FFPROBE_ENV)?;
        Ok(tools)
    }

    /// Uses these exact binaries, unchecked. For a bundled build that knows
    /// where its sidecars are.
    pub fn at(ffmpeg: impl Into<PathBuf>, ffprobe: impl Into<PathBuf>) -> Self {
        Self {
            ffmpeg: ffmpeg.into(),
            ffprobe: ffprobe.into(),
        }
    }

    /// A fresh `ffmpeg` command with no arguments.
    pub fn ffmpeg(&self) -> Command {
        Command::new(&self.ffmpeg)
    }

    /// A fresh `ffprobe` command with no arguments.
    pub fn ffprobe(&self) -> Command {
        Command::new(&self.ffprobe)
    }

    fn check(&self, binary: &PathBuf, env: &'static str) -> Result<(), ToolsError> {
        Command::new(binary)
            .arg("-version")
            .output()
            .map_err(|source| ToolsError::NotFound {
                binary: binary.clone(),
                env,
                source,
            })
            .map(|_| ())
    }
}

fn binary_from_env(variable: &str, fallback: &str) -> PathBuf {
    std::env::var_os(variable).map_or_else(|| PathBuf::from(fallback), PathBuf::from)
}

/// Why the external tools are unusable.
#[derive(Debug, thiserror::Error)]
pub enum ToolsError {
    #[error(
        "cannot run {}: {source}\n\
         install ffmpeg and put it on PATH, or set {env} to its location",
        binary.display()
    )]
    NotFound {
        binary: PathBuf,
        env: &'static str,
        #[source]
        source: std::io::Error,
    },
}
