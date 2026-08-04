//! A string that must not end up in a log.

use std::fmt;

/// An API key, and the promise that printing it prints nothing.
///
/// The wrapper exists for one reason: `Debug` is how a value reaches a log, a
/// panic message, an error report and a stack trace, and every one of those is
/// somewhere a key must never appear. A `String` would be printed by all of
/// them and nobody would notice until the log was somewhere public.
///
/// So the only way to the characters is [`Secret::expose`], named to be
/// uncomfortable — a call that reads as unusual at the one place it is
/// legitimate, which is building the request that spends the key.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    /// Wraps a key.
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// The key itself, for handing to the provider it belongs to.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Whether there is anything here at all.
    ///
    /// The one question about a key that can be answered without looking at
    /// it, and the one a settings screen needs: an empty value in a config
    /// file is a field somebody meant to fill in, not a credential.
    pub fn is_empty(&self) -> bool {
        self.0.trim().is_empty()
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(…)")
    }
}
