//! The rules, as arithmetic on one counter: no database, no clock of its own.
//!
//! Every time is seconds since the Unix epoch, handed in by the caller, so a
//! test walks a counter through a day of attempts without waiting for one.

/// Attempts one email, or one address, may make inside [`WINDOW`] before the
/// next is refused and a lock-out starts.
///
/// Ten: enough for a person who mistyped, tried the old password, then the
/// one before that — and for a household behind one address — while an
/// attacker gets ten guesses where they wanted millions.
pub const LIMIT: u32 = 10;

/// How long attempts are counted together: fifteen minutes. Past it, the
/// count starts again from nothing.
pub const WINDOW: i64 = 15 * 60;

/// The first lock-out: fifteen minutes. Each lock-out after it, with no quiet
/// day in between, is twice the one before.
pub const FIRST_LOCK: i64 = 15 * 60;

/// The longest a lock-out grows to: a day.
pub const LONGEST_LOCK: i64 = 24 * 60 * 60;

/// How long a counter must stay quiet — no attempt, no lock running — for its
/// strikes to be forgiven, and for the whole row to be forgotten.
pub const FORGIVEN_AFTER: i64 = 24 * 60 * 60;

/// One email's or one address's recent attempts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counter {
    /// Attempts counted since `window_start`.
    pub attempts: u32,
    /// When the current window began.
    pub window_start: i64,
    /// When the current lock-out ends; in the past (or 0) when there is none.
    pub locked_until: i64,
    /// Lock-outs in a row, which is what makes the next one longer.
    pub strikes: u32,
    /// The last attempt, refused ones included.
    pub touched: i64,
}

impl Counter {
    /// A counter that has seen nothing, as of `now`.
    pub fn new(now: i64) -> Self {
        Self {
            attempts: 0,
            window_start: now,
            locked_until: 0,
            strikes: 0,
            touched: now,
        }
    }

    /// Count one attempt at `now`, or refuse it with the seconds until the
    /// lock-out ends.
    ///
    /// Counted *before* the password is checked, so a burst of simultaneous
    /// guesses is counted as it arrives rather than after each one fails; a
    /// success gives its attempt back (`throttle::succeeded`).
    pub fn attempt(&mut self, now: i64) -> Result<(), i64> {
        if self.quiet_for(now) >= FORGIVEN_AFTER {
            self.strikes = 0;
        }
        self.touched = now;
        if self.locked_until > now {
            return Err(self.locked_until - now);
        }
        if now - self.window_start >= WINDOW {
            self.attempts = 0;
            self.window_start = now;
        }
        if self.attempts >= LIMIT {
            self.strikes = self.strikes.saturating_add(1);
            self.locked_until = now + lock_for(self.strikes);
            self.attempts = 0;
            self.window_start = now;
            return Err(self.locked_until - now);
        }
        self.attempts += 1;
        Ok(())
    }

    /// Seconds since anything happened to this counter: its last attempt, or
    /// the end of its lock-out, whichever is later.
    ///
    /// From the end of the lock and not its start, so a day-long lock-out
    /// does not forgive its own strikes the moment it ends.
    pub fn quiet_for(&self, now: i64) -> i64 {
        now - self.touched.max(self.locked_until)
    }
}

/// How long the lock-out that is strike number `strikes` lasts: fifteen
/// minutes, then thirty, an hour, two… up to [`LONGEST_LOCK`].
///
/// Grows because a fixed lock-out only sets a rate: ten guesses every fifteen
/// minutes is still close to a thousand a day against one account. Doubling
/// holds a patient attacker to about seventy guesses over the first day and a
/// half, and ten a day after that.
pub fn lock_for(strikes: u32) -> i64 {
    let doublings = strikes.saturating_sub(1).min(16);
    FIRST_LOCK.saturating_mul(1 << doublings).min(LONGEST_LOCK)
}
