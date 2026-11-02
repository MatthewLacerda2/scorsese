//! Locating ffmpeg and ffprobe, and building commands for them.
//!
//! This is the one place in the workspace that names those binaries. In
//! dev and CI they are on `PATH`; a bundled build points [`FFMPEG_ENV`] and
//! [`FFPROBE_ENV`] at its own copies (nothing bundles one yet: the supported
//! path is a clone with ffmpeg installed). Everything above this file asks
//! [`Tools`] for a command and never constructs one itself.
//!
//! It is also where every child process in the workspace is started
//! ([`child`]): with a cleared environment plus the few variables a tool
//! reads, so a child never holds the keys its parent does.
//!
//! And it is where a file is named to them as an input ([`input`]): through
//! the file protocol alone, never read as a URL or a list of other files.

pub mod input;

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::Command;

/// The variables a child keeps from its parent's environment, by name, and
/// every `LC_` one ([`kept`]). Where tools are and how they behave — never a
/// key. The web app's server holds its database password and every vendor's
/// key in its environment, and a decoder bug in the ffmpeg reading a member's
/// upload must reach none of them (#1656).
const KEPT: &[&str] = &[
    // Finding programs, and a home, a temporary folder, a locale and a clock.
    "PATH",
    "HOME",
    "TMPDIR",
    "TZ",
    "LANG",
    "LANGUAGE",
    // Where a self-built ffmpeg finds its shared libraries.
    "LD_LIBRARY_PATH",
    "DYLD_LIBRARY_PATH",
    "DYLD_FALLBACK_LIBRARY_PATH",
    // Fonts and caches, which ffmpeg's text and Chromium read.
    "FONTCONFIG_FILE",
    "FONTCONFIG_PATH",
    "XDG_CACHE_HOME",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_DATA_DIRS",
    "XDG_RUNTIME_DIR",
    // What a Windows program cannot start without.
    "SYSTEMROOT",
    "SYSTEMDRIVE",
    "WINDIR",
    "TEMP",
    "TMP",
    "PATHEXT",
    "COMSPEC",
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
    "PROGRAMDATA",
    // Where this workspace's own tools are, for a child that is this program
    // (`scorsese-server capture-one`), and how much it logs.
    FFMPEG_ENV,
    FFPROBE_ENV,
    crate::page::CHROME_ENV,
    crate::page::NO_SANDBOX_ENV,
    "RUST_LOG",
    // The Docker client the capture launcher runs.
    "DOCKER_HOST",
    "DOCKER_CONFIG",
    "DOCKER_CONTEXT",
    "DOCKER_CERT_PATH",
    "DOCKER_TLS_VERIFY",
    // Where an instrumented child writes its coverage.
    "LLVM_PROFILE_FILE",
];

/// A command for `program` that starts with an empty environment plus what
/// the allow-list keeps from this process's. The one way a child process is
/// started in this workspace: `std::process::Command::new` is a disallowed
/// method everywhere else (`clippy.toml`). Variables set on the returned
/// command with `env` still reach the child.
#[expect(
    clippy::disallowed_methods,
    reason = "the one place a child is started, and its environment is read to be filtered"
)]
pub fn child(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    command.env_clear().envs(kept(std::env::vars_os()));
    command
}

/// Those of `parent`'s variables a child keeps. Names are matched without
/// regard to case, which is how Windows reads them (`Path` is `PATH`).
fn kept(
    parent: impl IntoIterator<Item = (OsString, OsString)>,
) -> impl Iterator<Item = (OsString, OsString)> {
    parent.into_iter().filter(|(name, _)| {
        name.to_str().is_some_and(|name| {
            let name = name.to_ascii_uppercase();
            name.starts_with("LC_") || KEPT.contains(&name.as_str())
        })
    })
}

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
        child(&self.ffmpeg)
    }

    /// A fresh `ffprobe` command with no arguments.
    pub fn ffprobe(&self) -> Command {
        child(&self.ffprobe)
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

    /// Whether this ffmpeg was built with the encoder called `name`.
    ///
    /// Asked before a render in a codec that comes from an external library
    /// ([`crate::format::AudioCodec::library`]), because the ffmpeg on hand
    /// is a shipping decision rather than a constant: a distro build has
    /// `libmp3lame`, a minimal sidecar may not, and finding out from ffmpeg's
    /// own complaint part way into an encode is the expensive way to learn it.
    pub(crate) fn has_encoder(&self, name: &str) -> Result<bool, ToolsError> {
        let output = self
            .ffmpeg()
            .args(["-hide_banner", "-encoders"])
            .output()
            .map_err(|source| ToolsError::NotFound {
                binary: self.ffmpeg.clone(),
                env: FFMPEG_ENV,
                source,
            })?;
        Ok(lists_encoder(
            &String::from_utf8_lossy(&output.stdout),
            name,
        ))
    }

    fn check(&self, binary: &PathBuf, env: &'static str) -> Result<(), ToolsError> {
        self.run_version(binary, env).map(|_| ())
    }

    fn run_version(&self, binary: &PathBuf, env: &'static str) -> Result<Vec<u8>, ToolsError> {
        child(binary)
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

#[expect(
    clippy::disallowed_methods,
    reason = "SCORSESE_FFMPEG and SCORSESE_FFPROBE name a bundled build's binaries, not secrets"
)]
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

/// Whether `ffmpeg -encoders` lists `name`. Each encoder is a line of six
/// capability flags, the encoder's name, then a description — so the name is
/// the second word, matched whole: `mp3` must not be found in `libmp3lame`'s
/// line, nor `aac` in `aac_at`'s.
fn lists_encoder(listing: &str, name: &str) -> bool {
    listing
        .lines()
        .any(|line| line.split_whitespace().nth(1) == Some(name))
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
    use super::{Tools, child, kept, lists_encoder, version_from};
    use std::ffi::OsString;

    fn names(parent: &[&str]) -> Vec<String> {
        let parent = parent
            .iter()
            .map(|name| (OsString::from(name), OsString::from("x")));
        kept(parent)
            .map(|(name, _)| name.into_string().unwrap())
            .collect()
    }

    #[test]
    fn a_child_keeps_where_tools_are_and_nothing_else() {
        assert_eq!(
            names(&[
                "PATH",
                "LD_LIBRARY_PATH",
                "DATABASE_URL",
                "HOME",
                "ANTHROPIC_API_KEY",
                "LC_ALL",
                "STRIPE_SECRET_KEY",
                "SCORSESE_FFMPEG",
                "SCORSESE_STORAGE",
            ]),
            [
                "PATH",
                "LD_LIBRARY_PATH",
                "HOME",
                "LC_ALL",
                "SCORSESE_FFMPEG"
            ]
        );
    }

    #[test]
    fn a_name_is_kept_whatever_its_case_as_windows_reads_it() {
        assert_eq!(
            names(&["Path", "SystemRoot", "path_x"]),
            ["Path", "SystemRoot"]
        );
    }

    /// What the helper clears is the parent's environment, never what the
    /// caller sets on the command itself.
    #[cfg(unix)]
    #[test]
    fn a_variable_set_on_the_command_still_reaches_the_child() {
        let output = child("env").env("FONTS_HERE", "1").output().unwrap();
        let seen = String::from_utf8(output.stdout).unwrap();
        assert!(seen.contains("FONTS_HERE=1"), "{seen}");
    }

    /// What a child started as `ffmpeg` sees of its environment: `env` stands
    /// in for the binary and prints it.
    #[cfg(unix)]
    fn environment_of_ffmpeg() -> String {
        let output = Tools::at("env", "env").ffmpeg().output().unwrap();
        String::from_utf8(output.stdout).unwrap()
    }

    /// Cargo sets `CARGO_MANIFEST_DIR` in every test's environment, so it
    /// stands for a secret the parent holds (#1656): a decoder bug in ffmpeg
    /// must not reach the server's keys.
    #[cfg(unix)]
    #[test]
    #[expect(
        clippy::disallowed_methods,
        reason = "checks the parent really holds the variable the child must not see"
    )]
    fn ffmpeg_does_not_inherit_the_parents_environment() {
        assert!(std::env::var_os("CARGO_MANIFEST_DIR").is_some());
        let seen = environment_of_ffmpeg();
        assert!(
            !seen.contains("CARGO_MANIFEST_DIR="),
            "the child saw the parent's environment:\n{seen}"
        );
        assert!(seen.contains("PATH="), "the child lost PATH:\n{seen}");
    }

    #[test]
    fn an_encoder_is_found_by_its_whole_name_and_nothing_else() {
        let listing = "Encoders:\n V..... = Video\n ------\n \
                       A....D aac                  AAC (Advanced Audio Coding)\n \
                       A....D libmp3lame           libmp3lame MP3 (MPEG audio layer 3) (codec mp3)\n";
        assert!(lists_encoder(listing, "libmp3lame"));
        assert!(lists_encoder(listing, "aac"));
        assert!(
            !lists_encoder(listing, "mp3"),
            "the codec's name in a description is not an encoder"
        );
        assert!(!lists_encoder(listing, "wmav2"));
    }

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
