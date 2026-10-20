//! How often one caller may do something: a count per caller over the current
//! minute, the rest refused until the minute is out.
//!
//! Keyed by whatever names the caller — a user for web MCP's tool calls
//! (`http::mcp`), an address for the browser's crash reports
//! (`http::client_errors`), where nobody need be signed in. In memory: it
//! describes this process's last minute, which a restart may forget.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::db::UserId;

/// The window a limit counts over.
const MINUTE: Duration = Duration::from_secs(60);

/// Callers remembered before the ones whose minute is out are forgotten, so
/// a stream of new addresses cannot grow the map without end.
const REMEMBERED: usize = 4096;

/// How many times each caller has acted in the current minute.
#[derive(Debug, Clone)]
pub struct Limits<K = UserId> {
    per_minute: u32,
    windows: Arc<Mutex<HashMap<K, (Instant, u32)>>>,
}

impl<K: Eq + Hash> Limits<K> {
    /// At most `per_minute` a minute for each caller.
    pub fn new(per_minute: u32) -> Self {
        Self {
            per_minute,
            windows: Arc::default(),
        }
    }

    /// Count `count` more for `key`, or say how long until they may.
    pub(crate) fn take(&self, key: K, count: usize) -> Result<(), Duration> {
        if count == 0 {
            return Ok(());
        }
        let count = u32::try_from(count).unwrap_or(u32::MAX);
        let now = Instant::now();
        let mut windows = self.windows.lock().unwrap_or_else(PoisonError::into_inner);
        if windows.len() >= REMEMBERED && !windows.contains_key(&key) {
            windows.retain(|_, window| now.duration_since(window.0) < MINUTE);
        }
        let window = windows.entry(key).or_insert((now, 0));
        if now.duration_since(window.0) >= MINUTE {
            *window = (now, 0);
        }
        if window.1.saturating_add(count) > self.per_minute {
            return Err(MINUTE.saturating_sub(now.duration_since(window.0)));
        }
        window.1 += count;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_caller_has_a_minute_of_its_own() {
        let limits = Limits::new(2);
        assert_eq!(limits.take("a", 2), Ok(()));
        let wait = limits.take("a", 1).expect_err("a third in the minute");
        assert!(wait > Duration::ZERO && wait <= MINUTE);
        assert_eq!(limits.take("b", 1), Ok(()), "another caller is not held up");
        assert_eq!(
            limits.take("a", 0),
            Ok(()),
            "nothing counted is never refused"
        );
    }

    #[test]
    fn a_crowd_of_callers_forgets_only_the_ones_whose_minute_is_out() {
        let limits = Limits::new(1);
        for caller in 0..REMEMBERED {
            limits.take(caller, 1).unwrap();
        }
        // Everyone is still inside their minute: nobody is forgotten, so the
        // first is still refused.
        limits.take(REMEMBERED, 1).unwrap();
        assert!(limits.take(0, 1).is_err());
    }
}
