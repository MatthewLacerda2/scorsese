//! Browser sessions: a random cookie, looked up on every request.
//!
//! **Server-side and opaque**, rather than a signed cookie carrying the user:
//! logging out, an operator's password reset and deleting an account each end
//! a session *now*, by deleting its row, where a signed cookie stays valid
//! until it expires. It also means there is no signing key to keep secret.
//!
//! A session lasts [`LIFETIME_DAYS`] from login, and is not extended by use —
//! a fixed end is one less thing to reason about, and a month is long enough
//! that nobody logs in often.

use sqlx::postgres::PgPool;

use super::{AccountError, secret};
use crate::db::{self, UserId};

/// How long a login lasts, in days. The cookie's `Max-Age` says the same.
pub const LIFETIME_DAYS: i64 = 30;

/// Start a session for `user`, returning the cookie value to hand the
/// browser.
///
/// Also sweeps sessions that have expired — anybody's, so privileged. A
/// login is rare enough to carry that, and it keeps the table from holding
/// every session ever opened without a scheduled job to do it.
pub async fn open(pool: &PgPool, user: UserId) -> Result<String, AccountError> {
    let minted = secret::mint("")?;
    let mut sweep = db::privileged(pool).await?;
    sqlx::query("DELETE FROM sessions WHERE expires_at <= now()")
        .execute(&mut *sweep)
        .await?;
    sweep.commit().await?;

    let mut tx = db::scoped(pool, user).await?;
    sqlx::query(
        "INSERT INTO sessions (user_id, token_hash, expires_at)
         VALUES (member_id(), $1, now() + make_interval(days => $2::int))",
    )
    .bind(minted.digest)
    .bind(LIFETIME_DAYS)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(minted.plain)
}

/// Whose unexpired session `cookie` is, if anybody's.
///
/// Privileged: this is the step that finds out who is asking, so there is no
/// user to scope it to yet.
pub async fn find(pool: &PgPool, cookie: &str) -> Result<Option<UserId>, AccountError> {
    let mut tx = db::privileged(pool).await?;
    let id: Option<i64> = sqlx::query_scalar(
        "SELECT user_id FROM sessions WHERE token_hash = $1 AND expires_at > now()",
    )
    .bind(secret::digest(cookie))
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(id.map(UserId::from_row))
}

/// End the session `cookie` names, if it is `user`'s. Logging out.
pub async fn close(pool: &PgPool, user: UserId, cookie: &str) -> Result<(), AccountError> {
    let mut tx = db::scoped(pool, user).await?;
    sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(secret::digest(cookie))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}
