//! What the server is told by the environment it is started in.
//!
//! Four things, and only one of them is a secret. The database URL carries a
//! password, so it is held as a [`Secret`] — printing a [`Config`] prints
//! nothing of it, and no error below ever repeats its value.
//!
//! **Read through [`Environment`], the same value the provider-key resolver
//! reads.** That is the one-resolver rule of `docs/credentials.md` applied to
//! the server: an exported variable wins, a `.env` at or above the working
//! directory fills gaps and never overrides, and there is no third way a
//! setting reaches this process. The settings file — the second half of that
//! order — is deliberately not consulted for these: it is a desktop machine's
//! per-user file, and a server runs in a container that has no config
//! directory and is configured the way containers are, by its environment.
//! Provider keys the server spends with later go through
//! [`scorsese_providers::credentials::resolve`] unchanged; nothing here is a
//! second resolver for them.

use std::net::SocketAddr;
use std::path::PathBuf;

use scorsese_providers::credentials::{Environment, Secret};

use crate::storage::Storage;

/// Where the server's Postgres is, including its password.
pub const DATABASE_URL: &str = "DATABASE_URL";

/// The directory the server keeps users' files under. Must be absolute.
pub const STORAGE: &str = "SCORSESE_STORAGE";

/// The directory the server keeps what it can rebuild under — thumbnails,
/// uploads still arriving. Must be absolute, and outside [`STORAGE`]: the
/// library is backed up off the machine every night, and nothing here is
/// worth the bandwidth (`docs/web.md`, *Running the service*).
pub const CACHE: &str = "SCORSESE_CACHE";

/// The address the HTTP server listens on, e.g. `0.0.0.0:8080` in a container.
pub const BIND: &str = "SCORSESE_BIND";

/// Where the server listens when [`BIND`] is not set: this machine only.
///
/// Loopback by default because the safe mistake is a server nobody can reach,
/// not one the whole network can. A container opts out by setting the
/// variable, which is a decision somebody writes down in the compose file.
pub const DEFAULT_BIND: &str = "127.0.0.1:8080";

/// Everything the server needs to know before it starts.
#[derive(Debug, Clone)]
pub struct Config {
    /// The Postgres connection string. Never printed.
    pub database_url: Secret,
    /// The absolute directory users' files live under.
    pub storage: PathBuf,
    /// The absolute directory what can be rebuilt lives under.
    pub cache: PathBuf,
    /// The address to listen on.
    pub bind: SocketAddr,
}

/// Why the environment does not describe a server that can start.
///
/// Each variant names the variable, because a misconfigured container is fixed
/// by editing one line of a compose file and the message should say which.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// A variable the server cannot start without is unset or blank.
    #[error("{variable} is not set -- {purpose}")]
    Missing {
        /// The variable's name.
        variable: &'static str,
        /// What it is for, so the message says what to put there.
        purpose: &'static str,
    },

    /// A directory is a relative path.
    ///
    /// Refused rather than resolved against the working directory: a server's
    /// working directory is whatever its supervisor chose, and files landing
    /// somewhere that changes with how the process was launched is the
    /// relative-path bug #518 already paid for once.
    #[error("{variable} must be an absolute path, and {value:?} is not")]
    Relative {
        /// The variable's name.
        variable: &'static str,
        /// What it held.
        value: String,
    },

    /// The cache is the storage root or inside it, where the nightly backup
    /// would carry it off the machine.
    #[error("{CACHE} must be outside {STORAGE}, which is backed up; {value:?} is not")]
    CacheInStorage {
        /// What the cache variable held.
        value: String,
    },

    /// The listen address does not parse as `host:port`.
    #[error("{BIND} must be an address like {DEFAULT_BIND}, and {value:?} is not")]
    Bind {
        /// What the variable held.
        value: String,
    },
}

impl Config {
    /// The configuration this environment describes.
    ///
    /// A pure function of the value it is handed, so every rule here is
    /// testable without touching the process environment.
    pub fn from_environment(environment: &Environment) -> Result<Self, ConfigError> {
        let database_url = environment
            .get(DATABASE_URL)
            .map(Secret::new)
            .ok_or(ConfigError::Missing {
                variable: DATABASE_URL,
                purpose: "the Postgres connection string, e.g. postgres://user:password@host/scorsese",
            })?;

        let storage = directory(
            environment,
            STORAGE,
            "the absolute directory users' files are kept under",
        )?;
        let cache = directory(
            environment,
            CACHE,
            "the absolute directory thumbnails and unfinished uploads are kept under",
        )?;
        if cache.starts_with(&storage) {
            return Err(ConfigError::CacheInStorage {
                value: cache.display().to_string(),
            });
        }

        let bind = environment.get(BIND).unwrap_or(DEFAULT_BIND);
        let bind = bind.parse().map_err(|_| ConfigError::Bind {
            value: bind.to_owned(),
        })?;

        Ok(Self {
            database_url,
            storage,
            cache,
            bind,
        })
    }
}

impl Config {
    /// Where users' files go, kept and cached, as these settings say.
    pub fn files(&self) -> Storage {
        Storage::new(&self.storage, &self.cache)
    }
}

/// The absolute directory `variable` names.
fn directory(
    environment: &Environment,
    variable: &'static str,
    purpose: &'static str,
) -> Result<PathBuf, ConfigError> {
    let path = PathBuf::from(
        environment
            .get(variable)
            .ok_or(ConfigError::Missing { variable, purpose })?,
    );
    if !path.is_absolute() {
        return Err(ConfigError::Relative {
            variable,
            value: path.display().to_string(),
        });
    }
    Ok(path)
}
