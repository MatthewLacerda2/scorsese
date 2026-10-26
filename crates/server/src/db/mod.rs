//! The database: one pool, and the migrations that shape it.
//!
//! ## Migrations
//!
//! Versioned SQL files in `crates/server/migrations/`, compiled into the
//! binary by [`sqlx::migrate!`] and applied by [`migrate`] every time the
//! server starts, before it listens. So a deploy is "replace the binary and
//! restart": there is no separate migration step for somebody to forget, and
//! no container that serves requests against a schema older than its code.
//!
//! The rules a new migration follows, and why:
//!
//! - **Named `NNNN_what_it_does.sql`**, four digits, numbered one after the
//!   last with no gaps. Sequential rather than timestamped *because* branches
//!   here are written in parallel and merged one at a time: two branches that
//!   both add `0002_` collide when the second rebases, and `tests/migrations.rs`
//!   fails on the duplicate, which is the loud version of that conflict. A
//!   timestamp would let both through and apply them in an order neither
//!   author tested.
//! - **Never edited once merged.** sqlx records a checksum of every migration
//!   it applied and refuses to start against a database whose history
//!   disagrees with the binary. That refusal is correct: the production
//!   database holds people's work and their money, and the fix for a wrong
//!   migration is the next migration.
//! - **A file sqlx would skip is an error, not a no-op.** sqlx silently
//!   ignores a file whose name does not parse — `0002-users.sql` would simply
//!   never run. `tests/migrations.rs` holds every file in the directory to the
//!   naming rule so that a typo cannot become a table that does not exist.
//!
//! Forward-only: there are no down migrations. Undoing one on a database that
//! holds other people's data is a new migration written for the case in hand,
//! not a script written in advance for a case nobody has seen.
//!
//! ## Per-user isolation
//!
//! [`scope`] — how a query acts for one user and cannot see another's, and
//! what a new per-user table must look like. Read it before writing a
//! migration that adds a table.

pub mod scope;

use std::fmt::{self, Display};
use std::time::Duration;

use sqlx::migrate::{MigrateError, Migrator};
use sqlx::postgres::{PgPool, PgPoolOptions};

pub use scope::{Tx, UserId, member_pool, privileged, scoped};

use crate::config::Config;

/// Every migration in `crates/server/migrations/`, embedded at compile time.
pub static MIGRATOR: Migrator = sqlx::migrate!();

/// How long a request waits for a connection before giving up.
///
/// Short, because the one caller that meets this today is the health check,
/// and a health check that hangs for sqlx's default thirty seconds reads as
/// the server being down rather than the database.
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(3);

/// A pool connected to the configured database.
///
/// Connects eagerly: a server started against a database it cannot reach
/// fails here, at startup, with the reason — rather than coming up and
/// answering every request with an error.
pub async fn connect(config: &Config) -> Result<PgPool, sqlx::Error> {
    options().connect(config.database_url.expose()).await
}

/// The pool settings every connection in this server is made with.
pub fn options() -> PgPoolOptions {
    PgPoolOptions::new().acquire_timeout(ACQUIRE_TIMEOUT)
}

/// Bring the database's schema up to what this binary expects.
///
/// Idempotent: migrations already applied are skipped, so running this on
/// every start costs one query when there is nothing to do.
pub async fn migrate(pool: &PgPool) -> Result<(), MigrateError> {
    MIGRATOR.run(pool).await
}

/// What a user or an agent reads when the database fails under them: never
/// the detail, which is the operator's ([`user_facing`] logs it).
#[derive(Debug, Clone, Copy)]
pub enum Says {
    /// A request that wrote nothing worth worrying about: asking again is the
    /// answer.
    TryAgain,
    /// A job that moves money, named by what it was keeping the books on
    /// ("generation", "voice design"): whether it settled is the operator's
    /// to find out, so the user is not told to try again.
    Books(&'static str),
}

impl Display for Says {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TryAgain => f.write_str("the server's database failed; try again"),
            Self::Books(on) => write!(
                f,
                "the server failed while keeping the books on this {on}; the operator has the \
                 detail"
            ),
        }
    }
}

/// A database failure, logged as an error with its detail, `{context}:
/// {error}`, and what [`Says`] answered in its place — the one way every job
/// and tool reports one.
pub fn user_facing(context: &str, says: Says, error: impl Display) -> String {
    tracing::error!("{context}: {error}");
    says.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every reply a database failure gives, pinned to the words each job
    /// and tool said before they shared one helper (#1279).
    #[test]
    fn a_database_failure_says_what_it_always_said() {
        assert_eq!(
            user_facing("render job", Says::TryAgain, "boom"),
            "the server's database failed; try again"
        );
        assert_eq!(
            user_facing("generation job", Says::Books("generation"), "boom"),
            "the server failed while keeping the books on this generation; the operator has \
             the detail"
        );
        assert_eq!(
            user_facing("voice design job", Says::Books("voice design"), "boom"),
            "the server failed while keeping the books on this voice design; the operator has \
             the detail"
        );
    }
}
