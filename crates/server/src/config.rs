//! What the server is told by the environment it is started in.
//!
//! Eight things, and only one of them is a secret. The database URL carries a
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

use crate::http::client::Clients;
use crate::renders::Quota;
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

/// How much disk the finished-render cache aims to stay under, e.g. `20GB`.
/// A target rather than a wall — see [`crate::renders`].
pub const RENDER_QUOTA: &str = "SCORSESE_RENDER_QUOTA";

/// The render cache's quota when [`RENDER_QUOTA`] is not set.
pub const DEFAULT_RENDER_QUOTA: &str = "20GB";

/// The address the HTTP server listens on, e.g. `0.0.0.0:8080` in a container.
pub const BIND: &str = "SCORSESE_BIND";

/// The model the assistant (#540) runs on. Defaults to Claude Opus 5.5, and
/// must be a model `scorsese_providers::prices::claude` has a rate for — a
/// call nobody can price is a call nobody can charge for.
pub const ASSISTANT_MODEL: &str = "SCORSESE_ASSISTANT_MODEL";

/// The most one assistant turn may cost a user, in dollars, e.g. `2.50`.
/// Defaults to [`DEFAULT_TURN_CAP`].
pub const ASSISTANT_TURN_CAP: &str = "SCORSESE_ASSISTANT_TURN_CAP";

/// The per-turn cap when [`ASSISTANT_TURN_CAP`] is not set.
pub const DEFAULT_TURN_CAP: &str = "2.00";

/// Whether the server sits behind the deploy's nginx, and so believes the
/// client address it names in
/// [`CLIENT_HEADER`](crate::http::client::CLIENT_HEADER) — `true` or `false`, and
/// `false` when unset.
///
/// Only for a server whose port nothing but that nginx can reach, which is
/// what `deploy/compose.yaml` sets it for. Anywhere else a client could name
/// any address it liked, and dodge the login's brake or aim it at somebody
/// else ([`crate::http::client`]).
pub const TRUST_PROXY: &str = "SCORSESE_TRUST_PROXY";

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
    /// How much disk finished renders aim to stay under.
    pub render_quota: Quota,
    /// The address to listen on.
    pub bind: SocketAddr,
    /// The model the assistant runs on.
    pub assistant_model: String,
    /// The most one assistant turn may cost, in micro-dollars.
    pub assistant_turn_cap: i64,
    /// Which address a request is counted as coming from.
    pub clients: Clients,
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

    /// The render quota is not a size.
    #[error(
        "{RENDER_QUOTA} must be a size like {DEFAULT_RENDER_QUOTA} or 500MB, and {value:?} is not"
    )]
    Quota {
        /// What the variable held.
        value: String,
    },

    /// The listen address does not parse as `host:port`.
    #[error("{BIND} must be an address like {DEFAULT_BIND}, and {value:?} is not")]
    Bind {
        /// What the variable held.
        value: String,
    },

    /// The assistant's model has no published rate, so it cannot be charged.
    #[error("{ASSISTANT_MODEL} names {value:?}, which has no rate in prices::claude")]
    Model {
        /// What the variable held.
        value: String,
    },

    /// [`TRUST_PROXY`] is neither `true` nor `false`.
    #[error("{TRUST_PROXY} must be true or false, and {value:?} is neither")]
    TrustProxy {
        /// What the variable held.
        value: String,
    },

    /// The per-turn cap is not an amount of dollars above zero.
    #[error(
        "{ASSISTANT_TURN_CAP} must be dollars above zero like {DEFAULT_TURN_CAP}, and {value:?} is not"
    )]
    TurnCap {
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

        let quota = environment
            .get(RENDER_QUOTA)
            .unwrap_or(DEFAULT_RENDER_QUOTA);
        let render_quota = quota.parse().map_err(|()| ConfigError::Quota {
            value: quota.to_owned(),
        })?;

        let bind = environment.get(BIND).unwrap_or(DEFAULT_BIND);
        let bind = bind.parse().map_err(|_| ConfigError::Bind {
            value: bind.to_owned(),
        })?;

        let assistant_model = environment
            .get(ASSISTANT_MODEL)
            .unwrap_or(scorsese_providers::claude::MODEL)
            .to_owned();
        if scorsese_providers::prices::claude::rate(&assistant_model).is_none() {
            return Err(ConfigError::Model {
                value: assistant_model,
            });
        }
        let cap = environment
            .get(ASSISTANT_TURN_CAP)
            .unwrap_or(DEFAULT_TURN_CAP);
        let assistant_turn_cap = micro_dollars(cap).ok_or_else(|| ConfigError::TurnCap {
            value: cap.to_owned(),
        })?;

        let clients = match environment.get(TRUST_PROXY) {
            None | Some("false") => Clients::Peer,
            Some("true") => Clients::Proxy,
            Some(other) => {
                return Err(ConfigError::TrustProxy {
                    value: other.to_owned(),
                });
            }
        };

        Ok(Self {
            database_url,
            storage,
            cache,
            render_quota,
            bind,
            assistant_model,
            assistant_turn_cap,
            clients,
        })
    }
}

impl Config {
    /// Where users' files go, kept and cached, as these settings say.
    pub fn files(&self) -> Storage {
        Storage::new(&self.storage, &self.cache)
    }
}

/// Dollars written `2`, `2.5`, `$2.50` — at most six places, as micro-dollars
/// — when above zero.
fn micro_dollars(text: &str) -> Option<i64> {
    let text = text.trim().trim_start_matches('$');
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    let digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    if whole.is_empty() || !digits(whole) || !digits(fraction) || fraction.len() > 6 {
        return None;
    }
    let whole: i64 = whole.parse().ok()?;
    let fraction: i64 = format!("{fraction:0<6}").parse().ok()?;
    let micros = whole.checked_mul(1_000_000)?.checked_add(fraction)?;
    (micros > 0).then_some(micros)
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
