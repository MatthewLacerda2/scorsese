//! Where a provider key comes from, and what it is allowed to spend.
//!
//! One resolver, one order, one answer. A key is looked for in the
//! **environment** first — which is how a developer's exported variable and the
//! `.env` at the root of a checkout both work — and then in the **settings
//! file**, which is how a shipped build works, because an installed program has
//! no `.env` and never will.
//!
//! The order is that way round because an exported variable is somebody being
//! deliberate right now, and a settings file is what they decided once. The
//! failure this exists to prevent is two sources of truth: a key saved in the
//! window that `scorsese generate` cannot see, so the GUI works and the
//! terminal does not. The MCP server is the case that settles the shape — it
//! runs headless, will never see a settings screen, and has to find the key the
//! screen just saved.
//!
//! **A key never goes near `project.json`.** A project directory is promised to
//! survive being copied to another machine, and a credential inside one would
//! be copied with it.
//!
//! The ceiling lives here for the same reason the keys do: it is a fact about a
//! machine rather than about a video, and it is only meaningful beside the
//! credentials that make spending possible.

mod budget;
mod environment;
mod secret;
mod settings;

use std::path::PathBuf;

pub use budget::{Budget, OverBudget};
pub use environment::Environment;
pub use secret::Secret;
pub use settings::{Settings, SettingsError, path as settings_path};

/// Somebody scorsese pays to generate something.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    /// Google's Gemini API, which is where Veo is reached.
    Gemini,
    /// ElevenLabs, for narration.
    ElevenLabs,
}

impl Provider {
    /// The environment variable this provider's key is read from.
    pub const fn variable(self) -> &'static str {
        match self {
            Self::Gemini => "GEMINI_API_KEY",
            Self::ElevenLabs => "ELEVENLABS_API_KEY",
        }
    }

    /// What this provider is called when a message has to name it.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Gemini => "Gemini (Veo video)",
            Self::ElevenLabs => "ElevenLabs (narration)",
        }
    }

    /// This provider's key out of a settings file, if it has one there.
    fn in_settings(self, settings: &Settings) -> Option<&String> {
        match self {
            Self::Gemini => settings.gemini_api_key.as_ref(),
            Self::ElevenLabs => settings.elevenlabs_api_key.as_ref(),
        }
    }
}

/// Where a key was found.
///
/// Carried with the key because "it resolved" and "it resolved from the place
/// you think" are different claims, and the second is the one somebody debugging
/// a wrong key actually needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// An environment variable — exported, or filled in from a `.env`.
    Environment,
    /// The settings file at this path.
    Settings(PathBuf),
}

/// A key, and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// The key itself. Printing it prints nothing — see [`Secret`].
    pub secret: Secret,
    /// Which of the two places had it.
    pub source: Source,
}

/// Why a provider cannot be called.
#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    /// Neither place had a key.
    ///
    /// The message lists everywhere that was looked, because a missing key is
    /// almost always a key that is somewhere else — in the wrong file, in a
    /// shell that is not this one, under a name with a typo in it.
    #[error("no key for {provider}: looked in {places}")]
    Missing {
        /// Which provider has no key.
        provider: &'static str,
        /// Everywhere that was tried, in the order it was tried.
        places: String,
    },

    /// The settings file exists and could not be read.
    #[error(transparent)]
    Settings(#[from] SettingsError),
}

/// This provider's key, from the environment or the settings file.
///
/// The convenience form: reads the process environment, the nearest `.env` at
/// or above the working directory, and the settings file. Callers that would
/// rather be explicit — tests especially — build the two values and use
/// [`resolve_from`].
pub fn resolve(provider: Provider) -> Result<Resolved, CredentialError> {
    let here = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    resolve_from(provider, &Environment::discover(&here), &Settings::load()?)
}

/// This provider's key, out of exactly these two values.
///
/// The whole resolution order, as a pure function: no clock, no process
/// environment, no filesystem. That is what lets the order be tested at all —
/// the process environment is global mutable state shared with every thread,
/// and a test that has to set it cannot run beside another one.
pub fn resolve_from(
    provider: Provider,
    environment: &Environment,
    settings: &Settings,
) -> Result<Resolved, CredentialError> {
    if let Some(key) = environment.get(provider.variable()) {
        return Ok(Resolved {
            secret: Secret::new(key),
            source: Source::Environment,
        });
    }
    let from_settings = provider
        .in_settings(settings)
        .map(|key| Secret::new(key.trim()))
        .filter(|secret| !secret.is_empty());
    if let Some(secret) = from_settings {
        return Ok(Resolved {
            secret,
            source: Source::Settings(settings_path().unwrap_or_default()),
        });
    }
    Err(CredentialError::Missing {
        provider: provider.label(),
        places: looked_in(provider, environment),
    })
}

/// Everywhere a key was looked for, written the way a person would check them.
fn looked_in(provider: Provider, environment: &Environment) -> String {
    let mut places = vec![format!("the {} variable", provider.variable())];
    if let Some(dotenv) = environment.dotenv() {
        places.push(format!("{}", dotenv.display()));
    }
    match settings_path() {
        Ok(path) => places.push(format!("{}", path.display())),
        Err(error) => places.push(error.to_string()),
    }
    places.join(", ")
}
