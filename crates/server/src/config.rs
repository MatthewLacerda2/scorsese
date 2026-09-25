//! What the server is told by the environment it is started in.
//!
//! Three things, and only one of them is a secret. The database URL carries a
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

/// Where the server's Postgres is, including its password.
pub const DATABASE_URL: &str = "DATABASE_URL";

/// The directory the server keeps users' files under. Must be absolute.
pub const STORAGE: &str = "SCORSESE_STORAGE";

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

    /// The storage root is a relative path.
    ///
    /// Refused rather than resolved against the working directory: a server's
    /// working directory is whatever its supervisor chose, and files landing
    /// somewhere that changes with how the process was launched is the
    /// relative-path bug #518 already paid for once.
    #[error("{STORAGE} must be an absolute path, and {value:?} is not")]
    RelativeStorage {
        /// What the variable held.
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

        let storage = environment.get(STORAGE).ok_or(ConfigError::Missing {
            variable: STORAGE,
            purpose: "the absolute directory users' files are kept under",
        })?;
        let storage = PathBuf::from(storage);
        if !storage.is_absolute() {
            return Err(ConfigError::RelativeStorage {
                value: storage.display().to_string(),
            });
        }

        let bind = environment.get(BIND).unwrap_or(DEFAULT_BIND);
        let bind = bind.parse().map_err(|_| ConfigError::Bind {
            value: bind.to_owned(),
        })?;

        Ok(Self {
            database_url,
            storage,
            bind,
        })
    }
}
