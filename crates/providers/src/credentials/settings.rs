//! The settings file: where a shipped build keeps its keys and its ceiling.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The directory the settings file lives in, under the platform's config root.
const FOLDER: &str = "scorsese";

/// The settings file's name inside it.
const FILE: &str = "settings.json";

/// What one machine knows that no project does.
///
/// Keys and a spending ceiling, and deliberately nothing about any video: this
/// is a property of the person and the machine, not of the edit. Putting a key
/// in `project.json` would send it wherever the project went, and putting the
/// ceiling there would mean copying a project copied permission to spend.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Settings {
    /// The Gemini key, for video generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gemini_api_key: Option<String>,
    /// The ElevenLabs key, for narration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elevenlabs_api_key: Option<String>,
    /// The most one run may spend, in US cents.
    ///
    /// Absent means no ceiling, which is the state a fresh install is in — a
    /// limit nobody chose is not a limit, and inventing one would refuse the
    /// first honest run somebody made.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_cents: Option<u64>,
}

/// Why the settings file could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    /// The platform gave no config directory to put it in.
    #[error(
        "no config directory on this platform: set HOME (or XDG_CONFIG_HOME) \
         so scorsese has somewhere to keep its settings"
    )]
    NoConfigDir,

    /// The file is there and could not be read or written.
    #[error("settings file {path}: {source}")]
    Io {
        /// Which file.
        path: PathBuf,
        /// What the filesystem said.
        source: io::Error,
    },

    /// The file is there and is not settings.
    #[error("settings file {path} is not valid settings JSON: {source}")]
    Malformed {
        /// Which file.
        path: PathBuf,
        /// What the parser said.
        source: serde_json::Error,
    },
}

impl Settings {
    /// Reads the settings file, or the defaults when there is not one yet.
    ///
    /// A missing file is not an error: it is what every machine looks like
    /// before anyone has opened the settings screen, and the resolver above
    /// still has the environment to try.
    pub fn load() -> Result<Self, SettingsError> {
        let path = path()?;
        match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|source| SettingsError::Malformed { path, source }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(SettingsError::Io { path, source }),
        }
    }

    /// Writes the settings file, creating the directory if it is not there.
    ///
    /// Owner-only on Unix, set on the file itself rather than left to the
    /// umask: this file holds credentials, and a default that happens to be
    /// safe on one machine is not a guarantee on the next.
    pub fn save(&self) -> Result<PathBuf, SettingsError> {
        let path = path()?;
        let parent = path.parent().unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(|source| SettingsError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
        let text =
            serde_json::to_string_pretty(self).map_err(|source| SettingsError::Malformed {
                path: path.clone(),
                source,
            })?;
        fs::write(&path, text).map_err(|source| SettingsError::Io {
            path: path.clone(),
            source,
        })?;
        restrict(&path)?;
        Ok(path)
    }
}

/// Where the settings file is on this machine.
///
/// Reported by a command rather than left to be guessed: when a key does not
/// resolve, "which file was it looking in" is the first thing anybody needs,
/// and a path nobody can print is a support conversation.
pub fn path() -> Result<PathBuf, SettingsError> {
    Ok(config_root()
        .ok_or(SettingsError::NoConfigDir)?
        .join(FOLDER)
        .join(FILE))
}

/// The platform's directory for per-user configuration.
fn config_root() -> Option<PathBuf> {
    let var = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    let home = || var("HOME").map(PathBuf::from);

    if cfg!(target_os = "windows") {
        return var("APPDATA").map(PathBuf::from);
    }
    if cfg!(target_os = "macos") {
        return home().map(|home| home.join("Library").join("Application Support"));
    }
    var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| home().map(|home| home.join(".config")))
}

/// Makes the file readable by its owner and nobody else, where the platform
/// has a say in it.
#[cfg(unix)]
fn restrict(path: &Path) -> Result<(), SettingsError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|source| {
        SettingsError::Io {
            path: path.to_path_buf(),
            source,
        }
    })
}

#[cfg(not(unix))]
fn restrict(_path: &Path) -> Result<(), SettingsError> {
    Ok(())
}
