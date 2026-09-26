//! Per-user isolation, enforced by Postgres rather than remembered by code.
//!
//! **One user can never see or touch another's data** (#527), and the rule
//! has to survive somebody writing a new query at 2am and forgetting it. So
//! it is not a `WHERE user_id = $1` every query must remember: it is three
//! things that make forgetting fail loudly instead.
//!
//! 1. **Every per-user table has row-level security.** It carries
//!    `user_id BIGINT NOT NULL REFERENCES users ON DELETE CASCADE` and a
//!    policy `USING (user_id = (SELECT member_id()))`. `member_id()` is the
//!    user this transaction was opened for, and it *raises* when there is
//!    none. Queries inside a scope need no owner filter at all — the policy
//!    is the filter — and an insert names its owner as `member_id()`, so the
//!    user id never travels through query text where it could be the wrong
//!    one.
//! 2. **The server's connections sit in a role that may read nothing.**
//!    [`member_pool`] puts every connection in `scorsese_unscoped`, which is
//!    granted no table at all. [`scoped`] opens a transaction in
//!    `scorsese_member`, the role the policies bind. A handler that queries
//!    the pool directly gets `permission denied for table …` on the first
//!    run of the first test — even against an empty table, which is where a
//!    policy alone would have stayed silent.
//! 3. **A test holds every table to (1)** — `tests/isolation.rs` reads the
//!    catalog and fails on any table without the owner column, the cascade,
//!    row-level security or a policy that calls `member_id()`. So a new
//!    migration that forgets the pattern is caught the day it is written, not
//!    the day somebody notices a leak. The cascade in the same check is what
//!    makes account deletion complete by construction.
//!
//! The one way around it is [`privileged`], which drops back to the login
//! role, bypasses the policies, and is for the handful of queries that are
//! cross-user by nature: finding whose a session cookie or token is, logging
//! in, the operator's commands, and the job worker's claim and crash recovery
//! (`jobs::store` — once a job is claimed, the rest runs scoped as its
//! owner). Every call to it is a place a reviewer
//! reads twice; it is short on purpose, so `grep privileged` stays a short
//! list.
//!
//! **This guards against mistakes, not against SQL injection.** `SET ROLE
//! NONE` is open to any role, so code that can run arbitrary SQL can leave
//! the scope. Queries here bind their values; that is the defence against
//! injection, and this is the defence against forgetting.
//!
//! Why not the alternatives: scoping helpers alone leave the owner filter in
//! query text, where it is exactly as forgettable as it was; cross-user tests
//! alone only cover the endpoints somebody remembered to test. Both are still
//! worth having, and the endpoints do have cross-user tests — but as a second
//! line, not the only one.

use sqlx::postgres::{PgPool, Postgres};
use sqlx::{Executor, Transaction};

/// A transaction against the database, in whichever role opened it.
pub type Tx = Transaction<'static, Postgres>;

/// Who a request, a session or a token belongs to.
///
/// Only this crate makes one, from a row it read — so holding a `UserId`
/// means an account with that id existed when it was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserId(i64);

impl UserId {
    /// A user id read from the database.
    pub(crate) fn from_row(id: i64) -> Self {
        Self(id)
    }

    /// The number, for putting in a path or a response.
    pub fn get(self) -> i64 {
        self.0
    }
}

/// A transaction acting for `user`: every per-user table shows their rows and
/// only theirs, and a row inserted is refused unless it is theirs.
///
/// Sets both the role and the user, so it is correct on any pool — the
/// server's [`member_pool`] or the login role's own, as the operator
/// commands and the tests use. Both settings end with the transaction.
pub async fn scoped(pool: &PgPool, user: UserId) -> Result<Tx, sqlx::Error> {
    let mut tx = pool.begin().await?;
    // One round trip for both settings; `set_config('role', …, true)` is
    // `SET LOCAL ROLE` in a form that takes a bound parameter beside it.
    sqlx::query(
        "SELECT set_config('role', 'scorsese_member', true),
                set_config('scorsese.member', $1, true)",
    )
    .bind(user.0.to_string())
    .execute(&mut *tx)
    .await?;
    Ok(tx)
}

/// A transaction in the login role, which row-level security does not bind.
///
/// Only for what is cross-user by nature — see the module doc. Everything
/// that acts *for* a user, once it knows who, goes through [`scoped`].
pub async fn privileged(pool: &PgPool) -> Result<Tx, sqlx::Error> {
    let mut tx = pool.begin().await?;
    tx.execute("SET LOCAL ROLE NONE").await?;
    Ok(tx)
}

/// A pool on the same database as `pool` whose connections may read nothing
/// until a [`scoped`] or [`privileged`] transaction says otherwise.
///
/// What the server answers requests with. Connects eagerly, so a database
/// whose migrations have not created the roles fails at startup.
pub async fn member_pool(pool: &PgPool) -> Result<PgPool, sqlx::Error> {
    let options = (*pool.connect_options()).clone();
    super::options()
        .after_connect(|connection, _| {
            Box::pin(async move {
                connection
                    .execute("SET ROLE scorsese_unscoped")
                    .await
                    .map(drop)
            })
        })
        .connect_with(options)
        .await
}
