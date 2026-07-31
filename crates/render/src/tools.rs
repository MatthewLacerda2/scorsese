//! Locating ffmpeg and ffprobe, and building commands for them.
//!
//! This is the one place in the workspace that names those binaries. In
//! dev and CI they are on `PATH`; in a shipped build they are Tauri
//! sidecars next to the executable. Everything above this file asks
//! [`Tools`] for a command and never constructs one itself.

use std::path::PathBuf;
use std::process::Command;

/// Overrides where ffmpeg is found, so a bundled build can point at its
/// sidecar without changing any calling code.
pub const FFMPEG_ENV: &str = "SCORSESE_FFMPEG";

/// The ffprobe half of [`FFMPEG_ENV`]. Set separately because the two binaries
/// do not have to come from the same place.
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

    /// What this ffmpeg calls itself, e.g. `n8.1.2` or `6.1.1-3ubuntu5`.
    ///
    /// Worth having because ffmpeg is not only the encoder at the end of a
    /// render — it is the **decoder** at the start of one, and the frames the
    /// compositor works on are whatever it handed over. Two builds far enough
    /// apart can disagree about those, so anything holding pixels to a stored
    /// reference wants to be able to name which one produced them. The golden
    /// harness records this beside its references for exactly that.
    ///
    /// Reported verbatim rather than parsed into numbers: distributions add
    /// their own suffixes and builds add their own prefixes, and the useful
    /// question is "is this the same build" rather than "which is newer".
    pub fn ffmpeg_version(&self) -> Result<String, ToolsError> {
        let output = self.run_version(&self.ffmpeg, FFMPEG_ENV)?;
        Ok(version_from(&String::from_utf8_lossy(&output)))
    }

    fn check(&self, binary: &PathBuf, env: &'static str) -> Result<(), ToolsError> {
        self.run_version(binary, env).map(|_| ())
    }

    fn run_version(&self, binary: &PathBuf, env: &'static str) -> Result<Vec<u8>, ToolsError> {
        Command::new(binary)
            .arg("-version")
            .output()
            .map_err(|source| ToolsError::NotFound {
                binary: binary.clone(),
                env,
                source,
            })
            .map(|output| output.stdout)
    }
}

fn binary_from_env(variable: &str, fallback: &str) -> PathBuf {
    std::env::var_os(variable).map_or_else(|| PathBuf::from(fallback), PathBuf::from)
}

/// Pulls the version token out of `ffmpeg -version`, whose first line has read
/// `ffmpeg version <token> Copyright (c) …` for as long as the tool has
/// existed.
///
/// A line that does not have that shape falls back to the whole first line
/// rather than failing. This string is a record of what ran, and a slightly
/// verbose record is worth more than a render aborted over a banner.
fn version_from(output: &str) -> String {
    let first = output.lines().next().unwrap_or_default().trim();
    let mut words = first.split_whitespace();
    match (words.next(), words.next(), words.next()) {
        (Some("ffmpeg"), Some("version"), Some(token)) => token.to_owned(),
        _ => first.to_owned(),
    }
}

/// Why the external tools are unusable.
#[derive(Debug, thiserror::Error)]
pub enum ToolsError {
    #[error(
        "cannot run {}: {source}\n\
         install ffmpeg and put it on PATH, or set {env} to its location",
        binary.display()
    )]
    /// Nothing runnable at the path we resolved — not installed, not on
    /// `PATH`, or an override pointing somewhere stale.
    NotFound {
        /// The path that was tried.
        binary: PathBuf,
        /// The variable that would override it.
        env: &'static str,
        /// Why the process would not start.
        #[source]
        source: std::io::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::version_from;

    #[test]
    fn reads_the_version_a_distribution_build_reports() {
        // Ubuntu 24.04's package, which is what CI decodes with.
        let banner = "ffmpeg version 6.1.1-3ubuntu5 Copyright (c) 2000-2023 the FFmpeg developers\n\
                      built with gcc 13\n";
        assert_eq!(version_from(banner), "6.1.1-3ubuntu5");
    }

    #[test]
    fn reads_the_version_a_source_build_reports() {
        // Arch's, which prefixes the tag with `n`. The two are recorded
        // verbatim rather than normalised, because "is this the same build"
        // is the question and "which is newer" is not.
        let banner = "ffmpeg version n8.1.2 Copyright (c) 2000-2026 the FFmpeg developers\n";
        assert_eq!(version_from(banner), "n8.1.2");
    }

    #[test]
    fn an_unfamiliar_banner_is_kept_rather_than_discarded() {
        // A record of what ran is the point, so a banner that does not have
        // the expected shape is reported whole instead of failing the render
        // that asked.
        assert_eq!(
            version_from("something else entirely\n"),
            "something else entirely"
        );
        assert_eq!(version_from(""), "");
    }
}
