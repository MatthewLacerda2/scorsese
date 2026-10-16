//! What the operator's ElevenLabs plan allows, as the server is told it
//! (#1261): numbers that move with the plan, so read from the environment and
//! never written into the code that obeys them.

use scorsese_providers::credentials::Environment;

use crate::config::ConfigError;

/// How many ElevenLabs calls may be in flight at once, every kind of call
/// together ([`ELEVENLABS`](super::kinds::ELEVENLABS)).
pub const CONCURRENCY: &str = "SCORSESE_ELEVENLABS_CONCURRENCY";

/// [`CONCURRENCY`] when it is not set: Starter's 3 concurrent requests
/// (elevenlabs.io/pricing, read 2026-10-13). Free allows 2, and a bigger plan
/// more; the operator sets the plan's.
pub const DEFAULT_CONCURRENCY: usize = 3;

/// How many custom voices the account may hold.
pub const VOICE_SLOTS: &str = "SCORSESE_ELEVENLABS_VOICE_SLOTS";

/// [`VOICE_SLOTS`] when it is not set: Starter's 10 custom voices
/// (elevenlabs.io/pricing). Only voices made with Voice Design or Voice
/// Cloning take one; a Voice Library voice saved to the account does not
/// (elevenlabs.io/docs/eleven-creative/voices/voice-library, read
/// 2026-10-16).
pub const DEFAULT_VOICE_SLOTS: usize = 10;

/// The operator's ElevenLabs plan, as far as the server obeys it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElevenLabs {
    /// At most this many calls at once ([`CONCURRENCY`]).
    pub concurrency: usize,
    /// At most this many voices kept, every user's together ([`VOICE_SLOTS`]).
    pub voice_slots: usize,
}

impl Default for ElevenLabs {
    fn default() -> Self {
        Self {
            concurrency: DEFAULT_CONCURRENCY,
            voice_slots: DEFAULT_VOICE_SLOTS,
        }
    }
}

impl ElevenLabs {
    /// The plan `environment` describes, each number its default when unset.
    pub fn from_environment(environment: &Environment) -> Result<Self, ConfigError> {
        Ok(Self {
            concurrency: count(environment, CONCURRENCY, DEFAULT_CONCURRENCY)?,
            voice_slots: count(environment, VOICE_SLOTS, DEFAULT_VOICE_SLOTS)?,
        })
    }
}

/// The whole number above zero `variable` holds, or `default` when unset.
fn count(
    environment: &Environment,
    variable: &'static str,
    default: usize,
) -> Result<usize, ConfigError> {
    let Some(value) = environment.get(variable) else {
        return Ok(default);
    };
    value
        .trim()
        .parse()
        .ok()
        .filter(|count| *count > 0)
        .ok_or_else(|| ConfigError::Count {
            variable,
            value: value.to_owned(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(pairs: &[(&'static str, &'static str)]) -> Result<ElevenLabs, ConfigError> {
        ElevenLabs::from_environment(&Environment::of(pairs.iter().copied()))
    }

    #[test]
    fn unset_is_starters_plan() {
        assert_eq!(with(&[]).unwrap(), ElevenLabs::default());
        assert_eq!(ElevenLabs::default().concurrency, 3);
        assert_eq!(ElevenLabs::default().voice_slots, 10);
    }

    #[test]
    fn each_number_is_taken_when_given() {
        let plan = with(&[(CONCURRENCY, " 5 "), (VOICE_SLOTS, "30")]).unwrap();
        assert_eq!(plan.concurrency, 5);
        assert_eq!(plan.voice_slots, 30);
    }

    #[test]
    fn zero_or_not_a_number_is_named() {
        for (variable, value) in [
            (CONCURRENCY, "0"),
            (VOICE_SLOTS, "ten"),
            (CONCURRENCY, "-1"),
        ] {
            let error = with(&[(variable, value)]).unwrap_err().to_string();
            assert!(error.contains(variable) && error.contains(value), "{error}");
        }
    }
}
