//! What the server is told by the environment it is started in.
//!
//! Fourteen things, and four of them are secrets. The database URL carries a
//! password, the Google OAuth client has a secret and Stripe has two, so all
//! are held as a [`Secret`] — printing a [`Config`] prints nothing of them, and no error
//! below ever repeats their values.
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
use scorsese_providers::prices::parse_dollars;

use crate::assistant::DEFAULT_TURN_CAP_MICROS;
use crate::credits::dollars;
use crate::http::google::Google;
use crate::jobs::elevenlabs::ElevenLabs;
use crate::payments::Payments;
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

/// How much disk finished renders and page captures aim to stay under, e.g.
/// `20GB`.
/// A target rather than a wall — see [`crate::renders`].
pub const RENDER_QUOTA: &str = "SCORSESE_RENDER_QUOTA";

/// The render cache's quota when [`RENDER_QUOTA`] is not set.
pub const DEFAULT_RENDER_QUOTA: &str = "20GB";

/// The address the HTTP server listens on, e.g. `0.0.0.0:8080` in a container.
pub const BIND: &str = "SCORSESE_BIND";

/// The variable that once chose the assistant's one model, refused since
/// #705 made the model each project's choice ([`ConfigError::Model`]).
pub const RETIRED_MODEL: &str = "SCORSESE_ASSISTANT_MODEL";

/// The most one assistant turn on Sonnet 5.5 may cost a user, in dollars,
/// e.g. `2.50`; every other model's cap scales from it with its rates
/// (`Assistant::cap_micros`, #1461). Defaults to [`DEFAULT_TURN_CAP_MICROS`].
pub const ASSISTANT_TURN_CAP: &str = "SCORSESE_ASSISTANT_TURN_CAP";

/// The Google OAuth client's id, `….apps.googleusercontent.com` (#952).
pub const GOOGLE_CLIENT_ID: &str = "GOOGLE_OAUTH_CLIENT_ID";

/// The Google OAuth client's secret.
pub const GOOGLE_CLIENT_SECRET: &str = "GOOGLE_OAUTH_CLIENT_SECRET";

/// Stripe's secret key, `sk_test_…` or `sk_live_…` (#548): users buying
/// credits through Stripe Checkout.
pub const STRIPE_SECRET_KEY: &str = "STRIPE_SECRET_KEY";

/// The signing secret of the Stripe webhook endpoint, `whsec_…`, which proves
/// a delivery to `/api/payments/stripe/webhook` came from Stripe.
pub const STRIPE_WEBHOOK_SECRET: &str = "STRIPE_WEBHOOK_SECRET";

/// Where browsers reach the site, e.g. `https://scorseseestudio.com`: the
/// origin Google sends a sign-in back to, at `/api/auth/google/callback`.
///
/// Configured, never read off a request's `Host`: a header is whatever the
/// client typed, and a redirect URI built from it would send Google's code
/// wherever somebody pointed it.
pub const PUBLIC_ORIGIN: &str = "SCORSESE_PUBLIC_ORIGIN";

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
    /// The most one assistant turn may cost, in micro-dollars.
    pub assistant_turn_cap: i64,
    /// What the operator's ElevenLabs plan allows: calls at once, voices kept.
    pub elevenlabs: ElevenLabs,
    /// Sign in with Google, when [`GOOGLE_CLIENT_ID`], [`GOOGLE_CLIENT_SECRET`]
    /// and [`PUBLIC_ORIGIN`] are all set. `None` serves everything else and
    /// answers a sign-in with "not configured" — a development stack without
    /// a Google client still runs.
    pub google: Option<Google>,
    /// Users buying credits (#548), when [`STRIPE_SECRET_KEY`],
    /// [`STRIPE_WEBHOOK_SECRET`] and [`PUBLIC_ORIGIN`] are all set. `None`
    /// sells nothing and answers a purchase with "not configured".
    pub payments: Option<Payments>,
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

    /// `SCORSESE_ASSISTANT_MODEL` is set. It chose the one model every
    /// assistant turn ran on; since #705 each project chooses its own, and a
    /// setting that silently stopped meaning anything would leave an operator
    /// believing it still did.
    #[error(
        "{RETIRED_MODEL} is set to {value:?}, but the assistant's model is now chosen per project \
         in the chat panel (#705); remove the setting"
    )]
    Model {
        /// What the variable held.
        value: String,
    },

    /// [`PUBLIC_ORIGIN`] is not an origin a browser can be sent back to.
    #[error(
        "{PUBLIC_ORIGIN} must be an origin like https://scorseseestudio.com (no path), \
         and {value:?} is not"
    )]
    Origin {
        /// What the variable held.
        value: String,
    },

    /// A count of something the vendor allows is not a whole number above
    /// zero (`jobs::elevenlabs`).
    #[error("{variable} must be a whole number above zero, and {value:?} is not")]
    Count {
        /// The variable's name.
        variable: &'static str,
        /// What it held.
        value: String,
    },

    /// The per-turn cap is not an amount of dollars above zero.
    #[error(
        "{ASSISTANT_TURN_CAP} must be dollars above zero like {}, and {value:?} is not",
        dollars(DEFAULT_TURN_CAP_MICROS).trim_start_matches('$')
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

        if let Some(value) = environment.get(RETIRED_MODEL).filter(|v| !v.is_empty()) {
            return Err(ConfigError::Model {
                value: value.to_owned(),
            });
        }
        let assistant_turn_cap = match environment.get(ASSISTANT_TURN_CAP) {
            Some(cap) => parse_dollars(cap)
                .filter(|micros| *micros > 0)
                .ok_or_else(|| ConfigError::TurnCap {
                    value: cap.to_owned(),
                })?,
            None => DEFAULT_TURN_CAP_MICROS,
        };

        let elevenlabs = ElevenLabs::from_environment(environment)?;
        let google = Google::from_environment(environment)?;
        let payments = Payments::from_environment(environment)?;

        Ok(Self {
            database_url,
            storage,
            cache,
            render_quota,
            bind,
            assistant_turn_cap,
            elevenlabs,
            google,
            payments,
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
